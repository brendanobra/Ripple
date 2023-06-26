use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::{Mutex as TokioMutex, RwLock};

use dpab_core::{
    gateway::DpabDelegate,
    message::{DistributorSession, DpabError, DpabRequest, DpabRequestPayload},
    model::apps::AppsRequest,
};
use prost_types::Duration;
use queues::CircularBuffer;
use tokio::sync::mpsc::{Receiver, Sender};
use tonic::async_trait;
use tower::util::error::optional::None;
use tracing::{debug, info, info_span};

use crate::{
    gateway::appsanity_gateway::{AppsanityConfig, CloudServiceScopes, GrpcClientSession},
    util::service_util::create_grpc_client_session,
    util::{cloud_linchpin_monitor::CloudLinchpinMonitor, cloud_periodic_sync::CloudPeriodicSync},
};

use super::{
    appsanity_account_link::{AccountLinkDelegate, XvpServiceUrls},
    appsanity_advertising::AdvertisingDelegate,
    appsanity_auth::AuthDelegate,
    appsanity_discovery::DiscoveryService,
    appsanity_metrics::SiftService,
    appsanity_permission::PermissionDelegate,
    appsanity_privacy::PrivacyService,
    distp_secure_storage::DistPSecureStorageService,
    xvp_sync_and_monitor::SyncAndMonitorService,
};
use dpab_core::message::DpabRequestPayload::{AppMetric, BadgerMetric};

use super::catalog::appsanity_catalog::{CatalogService, CatalogServiceConfig};

#[async_trait]
pub trait AppsanityDelegate: Send + Sync {
    async fn handle(&mut self, request: DpabRequest);
}

pub struct AppsanityServiceResolver {
    permission_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    session_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    ad_platform_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    secure_storage_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    cloud_sync: CloudPeriodicSync,
    cloud_monitor: CloudLinchpinMonitor,
    pub cloud_services: AppsanityConfig,
    eos_rendered: Arc<TokioMutex<CircularBuffer<Value>>>,
    session_token: Arc<RwLock<String>>,
    catalog_service: Option<CatalogService>,
}

impl AppsanityServiceResolver {
    pub fn new(
        cloud_services: AppsanityConfig,
        cloud_sync: CloudPeriodicSync,
        cloud_monitor: CloudLinchpinMonitor,
        eos_rendered: Arc<TokioMutex<CircularBuffer<Value>>>,
        session_token: Arc<RwLock<String>>,
    ) -> AppsanityServiceResolver {
        let catalog_service = if cloud_services.catalog_service.enabled {
            Some(CatalogService::new(
                create_grpc_client_session(cloud_services.catalog_service.url.clone()),
                cloud_services.catalog_service.clone(),
            ))
        } else {
            None
        };

        AppsanityServiceResolver {
            permission_service_grpc_client_session: create_grpc_client_session(
                cloud_services.permission_service.url.clone(),
            ),
            session_service_grpc_client_session: create_grpc_client_session(
                cloud_services.session_service.url.clone(),
            ),
            ad_platform_service_grpc_client_session: create_grpc_client_session(
                cloud_services.ad_platform_service.url.clone(),
            ),
            secure_storage_service_grpc_client_session: create_grpc_client_session(
                cloud_services.secure_storage_service.url.clone(),
            ),
            cloud_sync,
            cloud_monitor,
            cloud_services,
            eos_rendered,
            session_token,
            catalog_service,
        }
    }

    pub fn init(&mut self, session: Option<DistributorSession>, dpab_req_tx: Sender<DpabRequest>) {
        if let Some(cs) = self.catalog_service.as_mut() {
            cs.init(session, dpab_req_tx);
        }
    }

    pub async fn resolve(&mut self, request: DpabRequest) -> Result<(), DpabError> {
        let parent_span = match request.parent_span.clone() {
            Some(s) => s,
            None => info_span!("appsanity service resolver"),
        };
        info_span!(parent: &parent_span, "appsanity service resolver", dpab_request=?request);
        match request.payload.clone() {
            AppMetric(_, _, session) => {
                let mut locked = self.session_token.write().await;
                *locked = session.token.clone();
            }
            BadgerMetric(_, _, session) => {
                let mut locked = self.session_token.write().await;
                *locked = session.token.clone();
            }
            _ => {}
        };

        if let DpabRequestPayload::Apps(_) = request.payload {
            if let Some(cs) = self.catalog_service.as_mut() {
                cs.handle(request).await;
            }
            return Ok(());
        }

        let handler: Option<Box<dyn DpabDelegate>> = match request.payload.clone() {
            DpabRequestPayload::SyncAndMonitor(_) => Some(Box::new(SyncAndMonitorService {
                cloud_periodic_sync: self.cloud_sync.clone(),
                cloud_linchpin_monitor: self.cloud_monitor.clone(),
                cloud_services: self.cloud_services.clone(),
            })),
            DpabRequestPayload::Advertising(_) => Some(Box::new(AdvertisingDelegate {
                ad_platform_service_grpc_client_session: self
                    .ad_platform_service_grpc_client_session
                    .clone(),
            })),
            DpabRequestPayload::Auth(_) => Some(Box::new(AuthDelegate {
                permission_service_grpc_client_session: self
                    .permission_service_grpc_client_session
                    .clone(),
            })),
            DpabRequestPayload::AccountLink(_) => Some(Box::new(AccountLinkDelegate {
                session_service_grpc_client_session: self
                    .session_service_grpc_client_session
                    .clone(),
                xvp_service_urls: XvpServiceUrls::new(
                    self.cloud_services.xvp_playback_service.url.clone(),
                    self.cloud_services.xvp_video_service.url.clone(),
                ),
                xvp_data_scopes: self.cloud_services.xvp_data_scopes.clone(),
            })),
            DpabRequestPayload::AppMetric(context, _, session) => {
                let ctx = match context {
                    Some(ctx) => Some(ctx.clone()),
                    None => None,
                };

                let sift_config = self.cloud_services.behavioral_metrics.sift.clone();

                let sift_service = SiftService::new(
                    sift_config.endpoint.clone(),
                    sift_config.batch_size,
                    sift_config.max_queue_size,
                    sift_config.metrics_schemas.clone(),
                    ctx,
                    self.eos_rendered.clone(),
                );
                Some(Box::new(sift_service))
            }
            DpabRequestPayload::BadgerMetric(context, _, _) => {
                let ctx = match context {
                    Some(ctx) => Some(ctx.clone()),
                    None => None,
                };

                let sift_config = self.cloud_services.behavioral_metrics.sift.clone();
                let sift_service = SiftService::new(
                    sift_config.endpoint.clone(),
                    sift_config.batch_size,
                    sift_config.max_queue_size,
                    sift_config.metrics_schemas.clone(),
                    ctx,
                    self.eos_rendered.clone(),
                );
                Some(Box::new(sift_service))
            }
            DpabRequestPayload::Permission(_) => Some(Box::new(PermissionDelegate {
                permission_service_grpc_client_session: self
                    .permission_service_grpc_client_session
                    .clone(),
            })),
            DpabRequestPayload::Discovery(_discovery_request) => {
                let discovery_service =
                    DiscoveryService::new(self.cloud_services.xvp_session_service.url.clone());
                Some(Box::new(discovery_service))
            }
            DpabRequestPayload::Privacy(privacy_request) => {
                let config = self.cloud_services.privacy_service.clone();
                let privacy_service = PrivacyService::new(
                    config.url.clone(),
                    privacy_request.get_session(),
                    &self.cloud_services.cloud_firebolt_mapping,
                );
                Some(Box::new(privacy_service))
            }
            DpabRequestPayload::SecureStorage(storage_request) => {
                let secure_storage_service = DistPSecureStorageService::new(
                    self.secure_storage_service_grpc_client_session.clone(),
                );
                Some(Box::new(secure_storage_service))
            }
            DpabRequestPayload::UserGrants(user_grant_request) => {
                let config = self.cloud_services.privacy_service.clone();
                let privacy_service = PrivacyService::new(
                    config.url.clone(),
                    user_grant_request.dist_session.clone(),
                    &self.cloud_services.cloud_firebolt_mapping,
                );
                Some(Box::new(privacy_service))
            }
            _ => None,
        };

        match handler {
            Some(mut delegate) => {
                delegate.handle(request).await;
                Ok(())
            }
            None => {
                debug!("No delegate found for default appsanity resolver");
                Err(DpabError::ServiceError)
            }
        }
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {

    use dpab_core::model::advertising::AdIdRequestParams;
    use dpab_core::model::advertising::AdInitObjectRequestParams;
    use dpab_core::model::advertising::AdvertisingRequest;
    use dpab_core::model::auth::AuthRequest;
    use dpab_core::model::auth::GetPlatformTokenParams;
    use dpab_core::{
        message::{DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse},
        model::auth,
    };
    use tokio::sync::{
        mpsc,
        oneshot::{self, Receiver, Sender},
    };

    use std::vec;
    use tonic::transport::ClientTlsConfig;
    // use dpab_core::model::auth::{AuthRequest, GetPermissionsParams};
    use std::{collections::HashMap, fmt::Debug};
    use tracing::Level;

    use crate::ad_platform::ad_platform_opt_out_service_client;
    use crate::gateway::appsanity_gateway::defaults;
    use crate::gateway::appsanity_gateway::AppsanityConfig;
    use crate::gateway::appsanity_gateway::MetricsSchemas;
    use crate::session_service;
    use crate::util::cloud_linchpin_monitor;
    use crate::util::cloud_periodic_sync;
    use crate::util::service_util::create_grpc_client_session;

    use super::*;

    #[tokio::test]
    async fn test_resolve_ad_app_sanity() {
        let mut passed = false;
        let (dpab_req_tx, dpab_req_rx) = mpsc::channel::<DpabRequest>(32);
        let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();

        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("tokens_ads"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let ad_id_request_params = AdIdRequestParams {
            privacy_data: HashMap::new(),
            app_id: String::from("App_id_1"),
            dist_session: get_distributer_session.clone(),
        };

        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::Advertising(AdvertisingRequest::GetAdIdObject(
                ad_id_request_params,
            )),
            callback: Some(dpab_res_tx),
            parent_span: None,
        };
        let th_req = tokio::spawn(async move {
            let (state_tx, mut state_rx) = mpsc::channel(32);
            let cloud_sync = CloudPeriodicSync::start(state_tx.clone());
            let cloud_monitor = CloudLinchpinMonitor::start(state_tx.clone());
            let _ = tokio::spawn(async move {
                while let Some(data) = state_rx.recv().await {
                    debug!("Received data: {:?}", data);
                }
            });
            let mut app_sanity_resolver = AppsanityServiceResolver::new(
                defaults(),
                cloud_sync,
                cloud_monitor,
                Arc::new(tokio::sync::Mutex::new(CircularBuffer::new(2))),
                Arc::new(tokio::sync::RwLock::new(String::from(""))),
            );
            app_sanity_resolver.resolve(get_dpab_request).await;
        });

        if let Ok(x) = dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }

    #[tokio::test]
    async fn test_resolve_auth_app_sanity() {
        let mut passed = false;
        let (dpab_req_tx, dpab_req_rx) = mpsc::channel::<DpabRequest>(32);
        let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();

        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_1"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::Auth(AuthRequest::GetPlatformToken(
                GetPlatformTokenParams {
                    app_id: String::from("App_1"),
                    dist_session: get_distributer_session.clone(),
                    content_provider: String::from("xumo"),
                    device_session_id: String::from("device_session_id"),
                    app_session_id: String::from("app_session_id"),
                },
            )),
            callback: Some(dpab_res_tx),
            parent_span: None,
        };
        let th_req = tokio::spawn(async move {
            let (state_tx, mut state_rx) = mpsc::channel(32);
            let cloud_sync = CloudPeriodicSync::start(state_tx.clone());
            let cloud_monitor = CloudLinchpinMonitor::start(state_tx.clone());
            let _ = tokio::spawn(async move {
                while let Some(data) = state_rx.recv().await {
                    debug!("Received data: {:?}", data);
                }
            });
            let mut app_sanity_resolver = AppsanityServiceResolver::new(
                defaults(),
                cloud_sync,
                cloud_monitor,
                Arc::new(tokio::sync::Mutex::new(CircularBuffer::new(2))),
                Arc::new(tokio::sync::RwLock::new(String::from(""))),
            );
            app_sanity_resolver.resolve(get_dpab_request).await;
        });

        if let Ok(x) = dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }

    #[tokio::test]
    async fn test_connection_failure() {
        let mut passed = false;
        let (dpab_req_tx, dpab_req_rx) = mpsc::channel::<DpabRequest>(32);
        let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();

        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_1"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::Auth(AuthRequest::GetPlatformToken(
                GetPlatformTokenParams {
                    app_id: String::from("App_1"),
                    dist_session: get_distributer_session.clone(),
                    content_provider: String::from("xumo"),
                    device_session_id: String::from("device_session_id"),
                    app_session_id: String::from("app_session_id"),
                },
            )),
            callback: Some(dpab_res_tx),
            parent_span: None,
        };
        let th_req = tokio::spawn(async move {
            let (state_tx, mut state_rx) = mpsc::channel(32);
            let cloud_sync = CloudPeriodicSync::start(state_tx.clone());
            let cloud_monitor = CloudLinchpinMonitor::start(state_tx.clone());
            let _ = tokio::spawn(async move {
                while let Some(data) = state_rx.recv().await {
                    debug!("Received data: {:?}", data);
                }
            });

            let mut app_sanity_resolver = AppsanityServiceResolver::new(
                defaults(),
                cloud_sync,
                cloud_monitor,
                Arc::new(tokio::sync::Mutex::new(CircularBuffer::new(2))),
                Arc::new(tokio::sync::RwLock::new(String::from(""))),
            );
            app_sanity_resolver.resolve(get_dpab_request).await;
        });

        if let Ok(x) = dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }
}
