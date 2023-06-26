use async_trait::async_trait;
use gateway::{DpabDelegate, Gateway, GatewayContext};
use message::DistributorSession;
use message::{DpabError, DpabRequest, DpabResponsePayload};
use model::{
    advertising::{AdIdResponse, AdInitObjectResponse, AdvertisingRequest},
    auth::AuthRequest,
    discovery::{
        ContentAccessResponse, DiscoveryAccountLinkRequest, DiscoveryRequest,
        EntitlementsAccountLinkResponse, LaunchPadAccountLinkResponse,
        MediaEventsAccountLinkResponse,
    },
    permissions::{GrantingPermissionsManager, PermissionsManager},
};
use serde_json::Value;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tracing::{info, info_span};

use crate::{message::DpabRequestPayload, model::metrics::LoggingBehavioralMetricsManager};

pub mod model {
    pub mod advertising;
    pub mod api_grants;
    pub mod apps;
    pub mod auth;
    pub mod badger;
    pub mod discovery;
    pub mod firebolt;
    pub mod metrics;
    pub mod permissions;
    pub mod privacy;
    pub mod secure_storage;
    pub mod sync_and_monitor;
    pub mod thor_permission_registry;
    pub mod user_grants;
}

pub mod gateway;
pub mod message;
/*
provide default clear sky (i.e. no cloudy) implementations */
#[cfg(not(appsanity))]
pub async fn run_dpab_local(
    session: Option<DistributorSession>,
    tx: Sender<DpabRequest>,
    rx: Receiver<DpabRequest>,
    config: Option<Value>,
) {
    let gateway_context = GatewayContext {
        session,
        sender: tx,
        receiver: rx,
        config: config,
    };
    Box::new(LocalDpabGateway {}).start(gateway_context).await;
}
#[cfg(not(appsanity))]
pub fn get_permissions_manager(
    _permissions_tx_channel: Sender<DpabRequest>,
) -> impl PermissionsManager + Send + Sync {
    GrantingPermissionsManager {}
}
#[cfg(not(appsanity))]
struct LocalDpabGateway {}
#[cfg(not(appsanity))]
impl LocalDpabGateway {
    pub async fn start_things(&self, gateway_context: GatewayContext) {
        let mut rx = gateway_context.receiver;
        let local_service_resolver = LocalServiceResolver {};

        let _ = tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let _ = local_service_resolver.resolve(req).await;
            }
        });
    }
}
#[async_trait]
impl Gateway for LocalDpabGateway {
    async fn start(mut self: Box<Self>, context: GatewayContext) -> Box<Self> {
        self.start_things(context).await;
        self
    }
    async fn shutdown(self: Box<Self>) -> bool {
        true
    }
}
struct LocalDpabDelegate {}
fn log(message: &Value) {
    info!("{}", message);
}
#[async_trait]
impl DpabDelegate for LocalDpabDelegate {
    async fn handle(&mut self, request: DpabRequest) {
        match &request.payload {
            DpabRequestPayload::Advertising(ad_request) => match ad_request {
                AdvertisingRequest::GetAdInitObject(init_request) => {
                    let params = init_request.clone();
                    request.respond_and_log(Ok(DpabResponsePayload::AdInitObject(
                        AdInitObjectResponse {
                            ad_server_url: "http://some.mock.ad.server.url".to_string(),
                            ad_server_url_template: "http://some.mock.ad.template.url".to_string(),
                            ad_network_id: "GAP".to_string(),
                            ad_profile_id: "slim".to_string(),
                            ad_site_section_id: "section1".to_string(),
                            ad_opt_out: false,
                            privacy_data: "very.private".to_string(),
                            ifa_value: "ifa.thing.happens".to_string(),
                            ifa: "ifa.fo.sho".to_string(),
                            app_name: params.durable_app_id.clone(),
                            app_bundle_id: "bundler".to_string(),
                            app_version: params.app_version.clone(),
                            distributor_app_id: params.distributor_app_id,
                            device_ad_attributes: "awesome,cool".to_string(),
                            coppa: "coppa.cabana".to_string(),
                            authentication_entity: "entity.of.auth".to_string(),
                        },
                    )));
                }
                AdvertisingRequest::GetAdIdObject(_) => {
                    request.respond_and_log(Ok(DpabResponsePayload::AdIdObject(AdIdResponse {
                        ifa: "ifa.tree.falls".to_string(),
                        ifa_type: "fake".to_string(),
                        lmt: "1+Maybe".to_string(),
                    })));
                }
                AdvertisingRequest::ResetAdIdentifier(_) => {
                    request.respond_and_log(Ok(DpabResponsePayload::None));
                }
            },
            DpabRequestPayload::Auth(auth) => match auth {
                AuthRequest::GetPlatformToken(_req) => {
                    let token =
                        "this.is.not.token. this is a mock. this is mocking you".to_string();
                    request.respond_and_log(Ok(DpabResponsePayload::String(token)));
                }
            },
            DpabRequestPayload::AppMetric(ctx, request, _session) => {
                LoggingBehavioralMetricsManager {
                    metrics_context: ctx.clone(),
                    log_fn: log,
                }
                .send_metric(request.clone())
                .await;
            }
            DpabRequestPayload::BadgerMetric(ctx, request, _session) => {
                LoggingBehavioralMetricsManager {
                    metrics_context: ctx.clone(),
                    log_fn: log,
                }
                .send_badger_metric(request.clone())
                .await;
            }
            DpabRequestPayload::Permission(_) => {
                request.respond_and_log(Ok(DpabResponsePayload::String("some_permission".into())));
            }

            DpabRequestPayload::AccountLink(account_link_req) => match account_link_req {
                DiscoveryAccountLinkRequest::EntitlementsAccountLink(_) => {
                    let response = EntitlementsAccountLinkResponse {};
                    request.respond_and_log(Ok(DpabResponsePayload::EntitlementsAccountLink(
                        response,
                    )));
                }
                DiscoveryAccountLinkRequest::MediaEventAccountLink(_) => {
                    let response = MediaEventsAccountLinkResponse {};
                    request
                        .respond_and_log(Ok(DpabResponsePayload::MediaEventsAccountLink(response)));
                }
                DiscoveryAccountLinkRequest::LaunchPadAccountLink(_) => {
                    let response = LaunchPadAccountLinkResponse {};
                    request
                        .respond_and_log(Ok(DpabResponsePayload::LaunchPadAccountLink(response)));
                }
                DiscoveryAccountLinkRequest::SignIn(_) => {
                    request.respond_and_log(Ok(DpabResponsePayload::None));
                }
            },
            DpabRequestPayload::Discovery(discovery_request) => match discovery_request {
                DiscoveryRequest::SetContentAccess(_) | DiscoveryRequest::ClearContent(_) => {
                    let response = ContentAccessResponse {};
                    request.respond_and_log(Ok(DpabResponsePayload::ContentAccess(response)));
                }
            },
            DpabRequestPayload::Privacy(_privacy_request) => {
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
            DpabRequestPayload::SecureStorage(secure_storage) => {
                info!("got secure storage request: {:?}", secure_storage);
                request.respond_and_log(Ok(DpabResponsePayload::None))
            }
            DpabRequestPayload::Apps(_) => {
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
            DpabRequestPayload::SyncAndMonitor(_) => {
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
            DpabRequestPayload::UserGrants(_) => {
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
        }
    }
}

struct LocalServiceResolver {}
/*return correct instance to handle request */
impl LocalServiceResolver {
    pub async fn resolve(&self, request: DpabRequest) -> Result<(), DpabError> {
        let parent_span = match request.parent_span.clone() {
            Some(s) => s,
            None => info_span!("appsanity service resolver"),
        };

        info_span!(parent: &parent_span, "local service resolver", dpab_request=?request);
        LocalDpabDelegate {}.handle(request).await;
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {

    use tokio::sync::oneshot;

    use crate::{
        gateway::DpabDelegate,
        message::{
            DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse, DpabResponsePayload,
        },
        model::discovery::{
            ClearContentSetParams, ContentAccessInfo, ContentAccessListSetParams,
            DiscoveryAccountLinkRequest, DiscoveryRequest, SessionParams, SignInRequestParams,
        },
        model::{
            privacy::PrivacyRequest,
            secure_storage::{
                SecureStorageGetRequest, SecureStorageRequest, SecureStorageSetRequest,
                StorageScope, StorageSetOptions,
            },
            sync_and_monitor::SyncAndMonitorRequest,
        },
        LocalDpabDelegate,
    };

    #[tokio::test]
    async fn test_privacy_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();
        let request = DpabRequest {
            payload: DpabRequestPayload::Privacy(PrivacyRequest::GetProperties(
                DistributorSession {
                    id: String::default(),
                    token: String::default(),
                    account_id: String::default(),
                    device_id: String::default(),
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }
    // Unit test for DiscoveryRequest::SetContentAccess
    #[tokio::test]
    async fn test_content_access_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();

        let dab_request = DpabRequest {
            payload: DpabRequestPayload::Discovery(DiscoveryRequest::SetContentAccess(
                ContentAccessListSetParams {
                    session_info: SessionParams {
                        app_id: "some_app".to_owned(),
                        dist_session: DistributorSession {
                            id: "id".to_owned(),
                            token: "token".to_owned(),
                            account_id: "account_id".to_owned(),
                            device_id: "device_id".to_owned(),
                        },
                    },
                    content_access_info: ContentAccessInfo {
                        availabilities: None,
                        entitlements: None,
                    },
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(dab_request).await;
        let resp = resp_rx.await;
        assert!(matches!(
            resp,
            Ok(Ok(DpabResponsePayload::ContentAccess(_)))
        ));
    }

    #[tokio::test]
    async fn test_clear_content_access_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();

        let dab_request = DpabRequest {
            payload: DpabRequestPayload::Discovery(DiscoveryRequest::ClearContent(
                ClearContentSetParams {
                    session_info: SessionParams {
                        app_id: "some_app".to_owned(),
                        dist_session: DistributorSession {
                            id: "id".to_owned(),
                            token: "token".to_owned(),
                            account_id: "account_id".to_owned(),
                            device_id: "device_id".to_owned(),
                        },
                    },
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(dab_request).await;
        let resp = resp_rx.await;
        assert!(matches!(
            resp,
            Ok(Ok(DpabResponsePayload::ContentAccess(_)))
        ));
    }
    #[tokio::test]
    async fn test_sign_in_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();

        let dab_request = DpabRequest {
            payload: DpabRequestPayload::AccountLink(DiscoveryAccountLinkRequest::SignIn(
                SignInRequestParams {
                    session_info: SessionParams {
                        app_id: "some_app".to_owned(),
                        dist_session: DistributorSession {
                            id: "id".to_owned(),
                            token: "token".to_owned(),
                            account_id: "account_id".to_owned(),
                            device_id: "device_id".to_owned(),
                        },
                    },
                    is_signed_in: true,
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(dab_request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }

    #[tokio::test]
    async fn test_secure_storage_get_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();

        let dab_request = DpabRequest {
            payload: DpabRequestPayload::SecureStorage(SecureStorageRequest::Get(
                SecureStorageGetRequest {
                    app_id: String::from("appid"),
                    scope: StorageScope::Account,
                    key: String::from("akey"),
                    distributor_session: DistributorSession {
                        id: String::from("foo"),
                        token: String::from("atoken"),
                        account_id: String::from("accountid"),
                        device_id: String::from("deviceid"),
                    },
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(dab_request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }

    #[tokio::test]
    async fn test_secure_storage_set_request() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();

        let dab_request = DpabRequest {
            payload: DpabRequestPayload::SecureStorage(SecureStorageRequest::Set(
                SecureStorageSetRequest {
                    app_id: String::from("appid"),
                    scope: StorageScope::Account,
                    key: String::from("akey"),
                    value: String::from("somevalue"),
                    options: Some(StorageSetOptions { ttl: 32 }),
                    distributor_session: DistributorSession {
                        id: String::from("foo"),
                        token: String::from("atoken"),
                        account_id: String::from("accountid"),
                        device_id: String::from("deviceid"),
                    },
                },
            )),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(dab_request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }
    #[tokio::test]
    async fn test_sync_and_monitor() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();
        let request = DpabRequest {
            payload: DpabRequestPayload::SyncAndMonitor(
                SyncAndMonitorRequest::UpdateDistributorToken("some_sat".to_owned()),
            ),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }
    #[tokio::test]
    async fn test_user_grants() {
        let mut handler = LocalDpabDelegate {};
        let (resp_tx, resp_rx) = oneshot::channel::<DpabResponse>();
        let request = DpabRequest {
            payload: DpabRequestPayload::UserGrants(crate::model::user_grants::UserGrantRequest {
                grant_entry: crate::model::user_grants::CloudGrantEntry {
                    role: crate::message::Role::Use,
                    capability: "xrn:firebolt:capability:device:model".to_owned(),
                    status: crate::model::user_grants::GrantStatus::Allowed,
                    last_modified_time: std::time::Duration::new(0, 0),
                    expiry_time: None,
                    app_name: None,
                },
                dist_session: DistributorSession {
                    id: String::from("foo"),
                    token: String::from("atoken"),
                    account_id: String::from("accountid"),
                    device_id: String::from("deviceid"),
                },
            }),
            callback: Some(resp_tx),
            parent_span: None,
        };
        handler.handle(request).await;
        let resp = resp_rx.await;
        assert!(matches!(resp, Ok(Ok(DpabResponsePayload::None))));
    }
}
