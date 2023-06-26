use std::{
    cmp,
    sync::{Arc, Mutex},
    time::SystemTime,
};

extern crate chrono;
use chrono::offset::Utc;
use chrono::DateTime;

use async_trait::async_trait;
use dpab_core::{
    gateway::DpabDelegate,
    message::{
        DistributorSession, DpabError, DpabRequest, DpabRequestPayload, DpabResponse,
        DpabResponsePayload,
    },
    model::apps::{AppMetadata, AppsRequest, AppsUpdate},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, oneshot};
use tonic::{transport::Channel, Request};
use tracing::{debug, error, info};

use crate::{
    catalog_service::{
        catalog_service_client::CatalogServiceClient, GetCatalogHashRequest,
        GetCatalogHashResponse, ListCatalogContentsRequest, ListCatalogContentsResponse,
    },
    gateway::appsanity_gateway::GrpcClientSession,
    util::service_util::decorate_request_with_session,
};

use super::catalog_persistence::CatalogHashStorage;

#[derive(Debug, Clone)]
pub enum CatalogRequest {
    GetCatalogHash(DistributorSession),
    ListCatalogContents(DistributorSession, String),
}

#[derive(Debug, Clone)]
pub enum CatalogResponse {
    CatalogHash(GetCatalogHashResponse),
    CatalogContents(ListCatalogContentsResponse),
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CatalogServiceConfig {
    pub enabled: bool,
    pub url: String,
    pub polling_interval_secs: u64,
    pub retry_interval_secs: u64,
    pub max_retry_interval_secs: u64,
    pub saved_dir: String,
}

struct PollingInterval {
    interval: u64,
    config: CatalogServiceConfig,
}

impl PollingInterval {
    pub fn new(config: CatalogServiceConfig) -> PollingInterval {
        PollingInterval {
            interval: config.polling_interval_secs,
            config,
        }
    }

    pub fn update(&mut self) -> u64 {
        if self.interval == self.config.polling_interval_secs {
            self.interval = self.config.retry_interval_secs;
        } else {
            self.interval = cmp::min(self.interval * 2, self.config.max_retry_interval_secs);
        }
        self.interval
    }

    pub fn get(&self) -> u64 {
        self.interval
    }
}

pub struct CatalogService {
    grpc_session: Arc<Mutex<GrpcClientSession>>,
    distributor_session: Option<DistributorSession>,
    config: CatalogServiceConfig,
    catalog_hash: CatalogHashStorage,
    listeners: Vec<Sender<AppsUpdate>>,
}

#[async_trait]
impl DpabDelegate for CatalogService {
    async fn handle(&mut self, dpab_request: DpabRequest) {
        debug!("handle: request={:?}", dpab_request);
        if let DpabRequestPayload::Apps(apps_request) = dpab_request.payload.clone() {
            match apps_request {
                AppsRequest::OnAppsUpdate(listener) => {
                    dpab_request.respond_and_log(self.add_listener(listener))
                }
                AppsRequest::RefreshSession(session) => {
                    self.distributor_session = Some(session.clone());
                    dpab_request.respond_and_log(Ok(DpabResponsePayload::None));
                }
                AppsRequest::CheckForUpdates => {
                    dpab_request.respond_and_log(self.check_for_updates().await);
                }
            }
        } else {
            error!("handle: Unexpected payload: {:?}", dpab_request.payload);
        }
    }
}

impl CatalogService {
    pub fn new(
        session: Arc<Mutex<GrpcClientSession>>,
        config: CatalogServiceConfig,
    ) -> CatalogService {
        CatalogService {
            grpc_session: session,
            distributor_session: None,
            config: config.clone(),
            catalog_hash: CatalogHashStorage::new(config.saved_dir),
            listeners: Vec::new(),
        }
    }

    pub fn init(&mut self, session: Option<DistributorSession>, dpab_req_tx: Sender<DpabRequest>) {
        self.distributor_session = session;
        let config = self.config.clone();

        // CS polling thread
        tokio::spawn(async move {
            let mut polling_interval = PollingInterval::new(config.clone());
            loop {
                info!("Polling catalog service...");

                let (cb_tx, cb_rx) = oneshot::channel::<DpabResponse>();
                let request = DpabRequest {
                    payload: DpabRequestPayload::Apps(AppsRequest::CheckForUpdates),
                    callback: Some(cb_tx),
                    parent_span: None,
                };

                if let Ok(_) = dpab_req_tx.send(request).await {
                    match cb_rx.await {
                        Ok(resp) => {
                            debug!("Got CS poll response: resp={:?}", resp);
                            if let Ok(r) = resp {
                                polling_interval = PollingInterval::new(config.clone());
                            } else {
                                polling_interval.update();
                            }
                        }
                        Err(e) => {
                            error!("Failed to receive catalog service poll request");
                            polling_interval.update();
                        }
                    }
                } else {
                    error!("Failed to send catalog service poll request");
                    polling_interval.update();
                }

                let duration = tokio::time::Duration::from_secs(polling_interval.get());
                tokio::time::sleep(duration).await;
            }
        });
    }

    fn get_channel(&self) -> Channel {
        self.grpc_session.lock().unwrap().get_grpc_channel().clone()
    }

    fn add_listener(&mut self, listener: Sender<AppsUpdate>) -> DpabResponse {
        debug!("add_listener: entry");
        self.listeners.push(listener);
        Ok(DpabResponsePayload::None)
    }

    async fn notify_listeners(&mut self, apps_update: AppsUpdate) {
        debug!("notify_listeners: entry");
        for listener in self.listeners.clone() {
            if let Err(e) = listener.send(apps_update.clone()).await {
                error!("notify_listeners: e={:?}", e);
            }
        }
    }

    async fn check_for_updates(&mut self) -> DpabResponse {
        debug!("check_for_updates: entry");
        let hash = self.get_catalog_hash().await?;
        if !hash.eq(&self.catalog_hash.get()) {
            info!("check_for_updates: Catalog hash changed");
            let catalog = self.list_catalog_contents(hash.clone()).await?;
            self.catalog_hash.set(hash);
            self.notify_listeners(AppsUpdate::new(catalog.clone()))
                .await;
            return DpabResponse::Ok(DpabResponsePayload::AppsUpdate(catalog));
        }
        debug!("check_for_updates: NONE");
        Ok(DpabResponsePayload::None)
    }

    async fn get_catalog_hash(&mut self) -> Result<String, DpabError> {
        debug!("get_catalog_hash: entry");

        if let None = self.distributor_session {
            error!("get_catalog_hash: No session");
            return Err(DpabError::ServiceError);
        }

        let distributor_session = self.distributor_session.clone().unwrap();

        let grpc_request = GetCatalogHashRequest {
            account_id: distributor_session.account_id.clone(),
            device_id: distributor_session.device_id.clone(),
            firebolt_version: String::from("0.1.0"), // TODO: Get real version.
        };

        let mut svc = CatalogServiceClient::with_interceptor(
            self.get_channel().clone(),
            |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &distributor_session);
                Ok(req)
            },
        );

        match svc.get_catalog_hash(grpc_request).await {
            Ok(resp) => {
                debug!("get_catalog_hash: resp={:?}", resp);
                let hash = resp.into_inner().catalog_hash;
                Ok(hash.clone())
            }
            Err(e) => {
                error!("get_catalog_hash: Could not retrieve hash: e={:?}", e);
                Err(DpabError::ServiceError)
            }
        }
    }

    async fn list_catalog_contents(
        &mut self,
        catalog_hash: String,
    ) -> Result<Vec<AppMetadata>, DpabError> {
        debug!("list_catalog_contents: catalog_hash={}", catalog_hash);

        if let None = self.distributor_session {
            error!("list_catalog_contents: No session");
            return Err(DpabError::ServiceError);
        }

        let distributor_session = self.distributor_session.clone().unwrap();
        let mut svc = CatalogServiceClient::with_interceptor(
            self.get_channel().clone(),
            |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &distributor_session);
                Ok(req)
            },
        );

        let mut catalog = Vec::new();
        let mut page_token = String::default();

        loop {
            let grpc_request = ListCatalogContentsRequest {
                catalog_hash: catalog_hash.clone(),
                page_token: page_token.clone(),
            };

            let resp = svc.list_catalog_contents(grpc_request).await;

            if let Err(e) = resp {
                error!("list_catalog_contents: Failed to get catalog: e={:?}", e);
                return Err(DpabError::ServiceError);
            }

            let resp = resp.unwrap().into_inner();

            for content in resp.catalog_contents {
                let item = AppMetadata::new(
                    content.durable_app_id,
                    content.title,
                    content.version,
                    content.location_uri,
                    None,
                );
                catalog.push(item);
            }

            if resp.next_page_token.is_empty() {
                return Ok(catalog);
            }

            page_token = resp.next_page_token;
        }
    }
}
