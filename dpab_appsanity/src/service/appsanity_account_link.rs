use crate::{
    client::{xvp_playback::XvpPlayback, xvp_videoservice::XvpVideoService},
    gateway::appsanity_gateway::{CloudServiceScopes, GrpcClientSession},
    session_service::{
        account_bridge_service_client::AccountBridgeServiceClient, AccountLinkAction,
        AccountLinkType, Entitlement, ImagesData, LinkAccountEntitlementsRequest,
        LinkAccountLaunchpad, LinkAccountLaunchpadRequest, LinkEntitlements,
    },
    util::service_util::decorate_request_with_session,
};
use async_trait::async_trait;
use dpab_core::{
    gateway::DpabDelegate,
    message::{DpabError, DpabRequest, DpabResponsePayload},
    model::discovery::{
        AccountLinkService, DiscoveryAccountLinkRequest, DiscoveryEntitlement,
        EntitlementsAccountLinkRequestParams, EntitlementsAccountLinkResponse,
        LaunchPadAccountLinkRequestParams, LaunchPadAccountLinkResponse,
        MediaEventsAccountLinkRequestParams, MediaEventsAccountLinkResponse, SignInRequestParams,
        ACCOUNT_LINK_ACTION_APP_LAUNCH, ACCOUNT_LINK_ACTION_CREATE, ACCOUNT_LINK_ACTION_DELETE,
        ACCOUNT_LINK_ACTION_SIGN_IN, ACCOUNT_LINK_ACTION_SIGN_OUT, ACCOUNT_LINK_TYPE_ACCOUNT_LINK,
        ACCOUNT_LINK_TYPE_ENTITLEMENT_UPDATES, ACCOUNT_LINK_TYPE_LAUNCH_PAD,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tonic::{transport::Channel, Request};
use tracing::{error, error_span, info, info_span, Instrument};

pub struct AppsanityAccountLinkService {
    supported_link_types: HashMap<String, AccountLinkType>,
    supported_link_action: HashMap<String, AccountLinkAction>,
    grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    xvp_service_urls: XvpServiceUrls,
    xvp_data_scopes: CloudServiceScopes,
}
#[derive(Debug, Clone)]
pub struct XvpServiceUrls {
    xvp_playback_base_url: String,
    xvp_video_service_base_url: String,
}

impl XvpServiceUrls {
    pub fn new(xvp_playback_base_url: String, xvp_video_service_base_url: String) -> Self {
        Self {
            xvp_playback_base_url,
            xvp_video_service_base_url,
        }
    }

    pub fn get_xvp_playback_url(&self) -> &str {
        &self.xvp_playback_base_url
    }
    pub fn get_xvp_video_service_url(&self) -> &str {
        &self.xvp_video_service_base_url
    }
}

impl AppsanityAccountLinkService {
    pub fn new(
        grpc_client_session: Arc<Mutex<GrpcClientSession>>,
        xvp_service_urls: XvpServiceUrls,
        xvp_data_scopes: CloudServiceScopes,
    ) -> Self {
        AppsanityAccountLinkService {
            supported_link_types: HashMap::from([
                (
                    ACCOUNT_LINK_TYPE_ACCOUNT_LINK.to_owned(),
                    AccountLinkType::AccountLink,
                ),
                (
                    ACCOUNT_LINK_TYPE_ENTITLEMENT_UPDATES.to_owned(),
                    AccountLinkType::EntitlementsUpdate,
                ),
                (
                    ACCOUNT_LINK_TYPE_LAUNCH_PAD.to_owned(),
                    AccountLinkType::LaunchPad,
                ),
            ]),
            supported_link_action: HashMap::from([
                (
                    ACCOUNT_LINK_ACTION_SIGN_IN.to_owned(),
                    AccountLinkAction::SignIn,
                ),
                (
                    ACCOUNT_LINK_ACTION_SIGN_OUT.to_owned(),
                    AccountLinkAction::SignOut,
                ),
                (
                    ACCOUNT_LINK_ACTION_APP_LAUNCH.to_owned(),
                    AccountLinkAction::AppLaunch,
                ),
                (
                    ACCOUNT_LINK_ACTION_CREATE.to_owned(),
                    AccountLinkAction::Create,
                ),
                (
                    ACCOUNT_LINK_ACTION_DELETE.to_owned(),
                    AccountLinkAction::Delete,
                ),
            ]),
            grpc_client_session,
            xvp_service_urls,
            xvp_data_scopes,
        }
    }

    pub fn get_link_types(&self, key: &String) -> AccountLinkType {
        *self
            .supported_link_types
            .get(key)
            .unwrap_or(&AccountLinkType::AccountLink)
    }

    pub fn get_link_action(&self, key: &String) -> AccountLinkAction {
        *self
            .supported_link_action
            .get(key)
            .unwrap_or(&AccountLinkAction::SignIn) // Todo find out the default.
    }

    pub fn get_entitlements(&self, entitlements: &Vec<DiscoveryEntitlement>) -> Vec<Entitlement> {
        let mut entitlement_map = Vec::new();

        let entitlements_iter = entitlements.iter();
        for item in entitlements_iter {
            entitlement_map.push(Entitlement {
                id: item.entitlement_id.to_owned(),
                start_date: item.start_time as i32,
                end_date: item.end_time as i32,
            })
        }
        entitlement_map
    }

    fn get_images(
        &self,
        images: &HashMap<String, HashMap<String, String>>,
    ) -> HashMap<String, ImagesData> {
        let mut image_map = HashMap::new();
        for (aspect, images_data) in images.iter() {
            for (locale, description) in images_data.iter() {
                image_map.insert(
                    aspect.to_string(),
                    ImagesData {
                        locale: locale.to_string(),
                        image_description: description.to_string(),
                    },
                );
            }
        }
        image_map
    }
}

#[async_trait]
impl AccountLinkService for AppsanityAccountLinkService {
    async fn entitlements_account_link(
        self: Box<Self>,
        request: DpabRequest,
        params: EntitlementsAccountLinkRequestParams,
    ) {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();

        let mut client =
            AccountBridgeServiceClient::with_interceptor(channel, |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &params.dist_session);
                Ok(req)
            });
        let perm_req = tonic::Request::new(LinkAccountEntitlementsRequest {
            link_entitlements: Some(LinkEntitlements {
                account_link_type: (params.account_link_type)
                    .map_or_else(|| -1, |x| self.get_link_types(&x) as i32),
                account_link_action: params
                    .account_link_action
                    .map_or_else(|| -1, |x| self.get_link_action(&x) as i32),
                entitlements: self.get_entitlements(&params.entitlements),
                durable_app_id: params.app_id.to_owned(),
            }),
            content_partner_id: params.content_partner_id,
        });
        match client.link_account_entitlements(perm_req).await {
            Ok(res) => {
                info!("entitlements_account_link SUCCESSSS !");
                let _res_i = res.into_inner();
                let response = EntitlementsAccountLinkResponse {};
                request.respond_and_log(Ok(DpabResponsePayload::EntitlementsAccountLink(response)));
            }
            Err(e) => {
                error!("Error Notifying Entitlements: {:?}", e);
                request.respond_and_log(Err(DpabError::IoError));
            }
        }
    }
    async fn media_events_account_link(
        self: Box<Self>,
        request: DpabRequest,
        params: MediaEventsAccountLinkRequestParams,
    ) {
        // let base_url = self.c
        match XvpPlayback::put_resume_point(
            self.xvp_service_urls.get_xvp_playback_url().to_string(),
            params,
        )
        .await
        {
            Ok(_) => {
                request.respond_and_log(Ok(DpabResponsePayload::MediaEventsAccountLink(
                    MediaEventsAccountLinkResponse {},
                )));
            }
            Err(_) => request.respond_and_log(Err(DpabError::ServiceError)),
        }
    }
    async fn launch_pad_account_link(
        self: Box<Self>,
        request: DpabRequest,
        params: LaunchPadAccountLinkRequestParams,
    ) {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();

        let mut client =
            AccountBridgeServiceClient::with_interceptor(channel, |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &params.dist_session);
                Ok(req)
            });
        let perm_req = tonic::Request::new(LinkAccountLaunchpadRequest {
            link_launchpad: Some(LinkAccountLaunchpad {
                expiration: params.link_launchpad.expiration,
                app_name: params.link_launchpad.app_name.to_owned(),
                content_id: if params.link_launchpad.content_id.is_some() {
                    params
                        .link_launchpad
                        .content_id
                        .as_ref()
                        .unwrap()
                        .to_string()
                } else {
                    "".to_owned()
                },
                deeplink: if params.link_launchpad.deeplink.is_some() {
                    params.link_launchpad.deeplink.as_ref().unwrap().to_string()
                } else {
                    "".to_owned()
                },
                content_url: if params.link_launchpad.content_url.is_some() {
                    params
                        .link_launchpad
                        .content_url
                        .as_ref()
                        .unwrap()
                        .to_string()
                } else {
                    "".to_owned()
                },
                durable_app_id: params.link_launchpad.app_id.to_owned(),
                title: params.link_launchpad.title.clone(),
                images: self.get_images(&params.link_launchpad.images),
                account_link_type: self.get_link_types(&params.link_launchpad.account_link_type)
                    as i32,
                account_link_action: self
                    .get_link_action(&params.link_launchpad.account_link_action)
                    as i32,
            }),
            content_partner_id: params.content_partner_id,
        });
        match client.link_account_launchpad(perm_req).await {
            Ok(res) => {
                info!("launch_pad_account_link SUCCESSSS !");
                let _res_i = res.into_inner();
                let response = LaunchPadAccountLinkResponse {};
                request.respond_and_log(Ok(DpabResponsePayload::LaunchPadAccountLink(response)));
            }
            Err(e) => {
                error!("Error Notifying launch_pad_account_link : {:?}", e);
                request.respond_and_log(Err(DpabError::IoError));
            }
        }
    }
    async fn sign_in(self: Box<Self>, request: DpabRequest, params: SignInRequestParams) {
        match XvpVideoService::sign_in(
            self.xvp_service_urls.get_xvp_video_service_url(),
            self.xvp_data_scopes.get_xvp_sign_in_state_scope(),
            params,
        )
        .await
        {
            Ok(_) => {
                info!("XVP SignIn returned success");
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
            Err(_) => {
                error!("Error in sending SignIn Information");
                request.respond_and_log(Err(DpabError::ServiceError));
            }
        }
    }
}
pub struct AccountLinkDelegate {
    pub session_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
    pub xvp_service_urls: XvpServiceUrls,
    pub xvp_data_scopes: CloudServiceScopes,
}

#[async_trait]
impl DpabDelegate for AccountLinkDelegate {
    async fn handle(&mut self, request: DpabRequest) {
        let parent_span = match request.parent_span.clone() {
            Some(s) => s,
            None => error_span!("appsanity account link delegate handling request"),
        };
        let span = info_span!(
            parent: &parent_span,
            "appsanity account link delegate handling request",
            ?request
        );
        let service = Box::new(AppsanityAccountLinkService::new(
            self.session_service_grpc_client_session.clone(),
            self.xvp_service_urls.clone(),
            self.xvp_data_scopes.clone(),
        ));
        match request.payload.as_account_link_request() {
            Some(req) => match req {
                DiscoveryAccountLinkRequest::MediaEventAccountLink(
                    media_event_account_link_params,
                ) => {
                    let future = async move {
                        service
                            .media_events_account_link(request, media_event_account_link_params)
                            .await;
                    };
                    tokio::spawn(future.instrument(span));
                }

                DiscoveryAccountLinkRequest::EntitlementsAccountLink(
                    entitlements_account_link_params,
                ) => {
                    let future = async move {
                        service
                            .entitlements_account_link(request, entitlements_account_link_params)
                            .await;
                    };
                    tokio::spawn(future.instrument(span));
                }

                DiscoveryAccountLinkRequest::LaunchPadAccountLink(
                    launchpad_account_link_params,
                ) => {
                    let future = async move {
                        service
                            .launch_pad_account_link(request, launchpad_account_link_params)
                            .await;
                    };
                    tokio::spawn(future.instrument(span));
                }
                DiscoveryAccountLinkRequest::SignIn(sign_in_params) => {
                    let future = async move {
                        service.sign_in(request, sign_in_params).await;
                    };
                    tokio::spawn(future.instrument(span));
                }
            },
            None => error!("Invalid dpab payload for AccountLink Service"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        client::xvp_playback::ResumePointResponse,
        client::xvp_videoservice::XvpVideoServiceResponse,
        gateway::appsanity_gateway::CloudServiceScopes,
        service::appsanity_account_link::{
            AccountLinkDelegate, AppsanityAccountLinkService, XvpServiceUrls,
        },
        session_service::{
            account_bridge_service_server::{AccountBridgeService, AccountBridgeServiceServer},
            LinkAccountEntitlementsRequest, LinkAccountEntitlementsResponse,
            LinkAccountLaunchpadRequest, LinkAccountLaunchpadResponse,
            LinkAccountMediaEventRequest, LinkAccountMediaEventResponse,
        },
    };

    use crate::util::service_util::create_grpc_client_session;
    use dpab_core::{
        gateway::DpabDelegate,
        message::{DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse},
        model::discovery::{
            AccountLaunchpad, DiscoveryAccountLinkRequest, DiscoveryEntitlement,
            EntitlementsAccountLinkRequestParams, LaunchPadAccountLinkRequestParams, MediaEvent,
            MediaEventsAccountLinkRequestParams, ProgressUnit, SessionParams, SignInRequestParams,
            ACCOUNT_LINK_ACTION_CREATE, ACCOUNT_LINK_TYPE_ENTITLEMENT_UPDATES,
            ACCOUNT_LINK_TYPE_LAUNCH_PAD, PROGRESS_UNIT_PERCENT,
        },
    };
    use httpmock::prelude::*;
    use std::{
        collections::{HashMap, HashSet},
        thread, time,
    };
    use tokio::sync::oneshot::{self, Receiver, Sender};
    use tonic::{transport::Server, Request, Response};

    pub mod resapi_service {
        tonic::include_proto!("ottx.resapi");
    }

    #[derive(Debug, Default)]
    pub struct AccountBridgeTestService {}

    #[tonic::async_trait]
    impl AccountBridgeService for AccountBridgeTestService {
        async fn link_account_entitlements(
            &self,
            request: Request<LinkAccountEntitlementsRequest>,
        ) -> Result<tonic::Response<LinkAccountEntitlementsResponse>, tonic::Status> {
            print!("Got a request: {:?} ", request);
            let _req = request.into_inner();

            let reply = LinkAccountEntitlementsResponse {};

            Ok(Response::new(reply))
        }

        async fn link_account_media_event(
            &self,
            request: Request<LinkAccountMediaEventRequest>,
        ) -> Result<tonic::Response<LinkAccountMediaEventResponse>, tonic::Status> {
            print!("Got a request: {:?} ", request);
            let _req = request.into_inner();

            let reply = LinkAccountMediaEventResponse {};

            Ok(Response::new(reply))
        }

        async fn link_account_launchpad(
            &self,
            request: Request<LinkAccountLaunchpadRequest>,
        ) -> Result<tonic::Response<LinkAccountLaunchpadResponse>, tonic::Status> {
            print!("Got a request: {:?} ", request);
            let _req = request.into_inner();

            let reply = LinkAccountLaunchpadResponse {};

            Ok(Response::new(reply))
        }
    }

    pub struct DpabTest {
        pub dpab_res_tx: Sender<DpabResponse>,
        pub dpab_res_rx: Receiver<DpabResponse>,
    }
    impl DpabTest {
        pub fn new() -> DpabTest {
            let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();
            DpabTest {
                dpab_res_tx,
                dpab_res_rx,
            }
        }
    }

    pub async fn start_mock_resapi_server() {
        let addr = "[::1]:50051".parse().unwrap();
        let account_bridge_service = AccountBridgeTestService::default();
        //let (_tx, rx) = oneshot::channel::<()>();

        let _server_future = tokio::spawn(async move {
            Server::builder()
                .add_service(AccountBridgeServiceServer::new(account_bridge_service))
                .serve(addr)
                .await
                .unwrap();
        });
    }

    #[tokio::test]
    pub async fn test_entitlements_account_link() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("xglobal"),
            token: String::from("RmlyZWJvbHQgTWFuYWdlIFNESyBSb2NrcyEhIQ=="),
            account_id: String::from("0123456789"),
            device_id: String::from("9876543210"),
        };

        let request_params = EntitlementsAccountLinkRequestParams {
            account_link_type: Some(ACCOUNT_LINK_TYPE_ENTITLEMENT_UPDATES.to_owned()),
            account_link_action: None,
            entitlements: vec![DiscoveryEntitlement {
                entitlement_id: String::from("123"),
                start_time: 1735689600,
                end_time: 1735689600,
            }],
            app_id: String::from("ABCDEF"),
            content_partner_id: String::from("STUVW"),
            dist_session: get_distributer_session.clone(),
        };
        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::AccountLink(
                DiscoveryAccountLinkRequest::EntitlementsAccountLink(request_params.clone()),
            ),
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };

        let test_channel =
            create_grpc_client_session(String::from("res-api.svc-qa.thor.comcast.com"));

        let mut account_link_delegate = AccountLinkDelegate {
            session_service_grpc_client_session: test_channel,
            xvp_service_urls: XvpServiceUrls::new(
                String::from("https://example.com"),
                String::from("https://example.com"),
            ),
            xvp_data_scopes: CloudServiceScopes::default(),
        };
        account_link_delegate.handle(get_dpab_request).await;

        if let Ok(_x) = dpab_test.dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }

    #[tokio::test]
    pub async fn test_media_events_account_link() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("xglobal"),
            token: String::from("RmlyZWJvbHQgTWFuYWdlIFNESyBSb2NrcyEhIQ=="),
            account_id: String::from("0123456789"),
            device_id: String::from("9876543210"),
        };
        let request_params = MediaEventsAccountLinkRequestParams {
            media_event: MediaEvent {
                content_id: String::from("partner.com/entity/123"),
                completed: true,
                progress: 50.0,
                progress_unit: ProgressUnit::Percent,
                watched_on: Some(String::from("2021-04-23T18:25:43.511Z")),
                app_id: String::from("ABCDEF"),
            },
            content_partner_id: String::from("STUVW"),
            client_supports_opt_out: false,
            data_tags: HashSet::default(),
            dist_session: get_distributer_session.clone(),
        };
        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::AccountLink(
                DiscoveryAccountLinkRequest::MediaEventAccountLink(request_params.clone()),
            ),
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };

        let test_channel =
            create_grpc_client_session(String::from("res-api.svc-qa.thor.comcast.com"));
        let server = MockServer::start();
        let resp = ResumePointResponse {
            message_id: Some(String::from("mid")),
            sns_status_code: Some(200),
            sns_status_text: Some(String::from("text")),
            aws_request_id: Some(String::from("arid")),
        };
        let xvp_mock = server.mock(|when, then| {
            when.method(PUT).path("/v1/partners/xglobal/accounts/0123456789/devices/9876543210/resumePoints/ott/STUVW/partner.com%2Fentity%2F123").query_param("clientId", "ripple");
            then.status(200)
                .header("content-type", "application/javascript")
                .body(serde_json::to_string(&resp).unwrap());
        });

        let mut account_link_delegate = AccountLinkDelegate {
            session_service_grpc_client_session: test_channel,
            xvp_service_urls: XvpServiceUrls::new(
                server.url("/v1"),
                String::from("https://example.com"),
            ),
            xvp_data_scopes: CloudServiceScopes::default(),
        };
        account_link_delegate.handle(get_dpab_request).await;

        if let Ok(x) = dpab_test.dpab_res_rx.await {
            passed = x.is_ok();
        }
        assert!(passed);
        xvp_mock.assert();
    }

    #[tokio::test]
    pub async fn test_launch_pad_account_link() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("xglobal"),
            token: String::from("RmlyZWJvbHQgTWFuYWdlIFNESyBSb2NrcyEhIQ=="),
            account_id: String::from("0123456789"),
            device_id: String::from("9876543210"),
        };
        let request_params = LaunchPadAccountLinkRequestParams {
            link_launchpad: AccountLaunchpad {
                expiration: 1735689600,
                app_name: String::from("Netflix"),
                content_id: Some(String::from("partner.com/entity/123")),
                deeplink: None,
                content_url: None,
                app_id: String::from("ABCDEF"),
                title: HashMap::from([(String::from("en"), String::from("Test Description"))]),
                images: HashMap::new(),
                account_link_type: ACCOUNT_LINK_TYPE_LAUNCH_PAD.to_owned(),
                account_link_action: ACCOUNT_LINK_ACTION_CREATE.to_owned(),
            },
            content_partner_id: String::from("STUVW"),
            dist_session: get_distributer_session.clone(),
        };
        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::AccountLink(
                DiscoveryAccountLinkRequest::LaunchPadAccountLink(request_params.clone()),
            ),
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };

        let test_channel =
            create_grpc_client_session(String::from("res-api.svc-qa.thor.comcast.com"));

        let mut account_link_delegate = AccountLinkDelegate {
            session_service_grpc_client_session: test_channel,
            xvp_service_urls: XvpServiceUrls::new(
                String::from("https://example.com"),
                String::from("https://example.com"),
            ),
            xvp_data_scopes: CloudServiceScopes::default(),
        };
        account_link_delegate.handle(get_dpab_request).await;

        if let Ok(_x) = dpab_test.dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }

    #[tokio::test]
    pub async fn test_sign_in() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("app1"),
            token: String::from("RmlyZWJvbHQgTWFuYWdlIFNESyBSb2NrcyEhIQ=="),
            account_id: String::from("0123456789"),
            device_id: String::from("9876543210"),
        };
        let request_params = SignInRequestParams {
            session_info: SessionParams {
                app_id: String::from("app1"),
                dist_session: get_distributer_session.clone(),
            },
            is_signed_in: true,
        };
        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::AccountLink(DiscoveryAccountLinkRequest::SignIn(
                request_params.clone(),
            )),
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };

        let test_channel =
            create_grpc_client_session(String::from("res-api.svc-qa.thor.comcast.com"));
        let server = MockServer::start();
        let resp = XvpVideoServiceResponse {
            partner_id: Some("app1".to_owned()),
            account_id: Some("0123456789".to_owned()),
            owner_reference: Some("xrn:xcal:subscriber:account:0123456789".to_owned()),
            entity_urn: Some("xrn:xvp:application:app1".to_owned()),
            entity_id: Some("app1".to_owned()),
            entity_type: Some("application".to_owned()),
            durable_app_id: Some("app1".to_owned()),
            event_type: Some("signIn".to_owned()),
            is_signed_in: Some(true),
            added: Some("2023-03-09T23:01:06.397Z".to_owned()),
            updated: Some("2023-03-13T17:32:20.610478364Z".to_owned()),
        };

        let xvp_session_mock = server.mock(|when, then| {
            when.method(PUT).path("/base_url/v1/partners/app1/accounts/0123456789/videoServices/xrn:xvp:application:app1/engaged")
            .query_param("ownerReference", "xrn:xcal:subscriber:account:0123456789")
            .query_param("clientId", "ripple");
            then.status(200)
                .header("content-type", "application/javascript")
                .body(serde_json::to_string(&resp).unwrap());
        });

        let mut account_link_delegate = AccountLinkDelegate {
            session_service_grpc_client_session: test_channel,
            xvp_service_urls: XvpServiceUrls::new(
                String::from("https://example.com"),
                server.url("/base_url/v1"),
            ),
            xvp_data_scopes: CloudServiceScopes::default(),
        };
        account_link_delegate.handle(get_dpab_request).await;

        if let Ok(x) = dpab_test.dpab_res_rx.await {
            passed = x.is_ok();
        }
        assert!(passed);
        xvp_session_mock.assert();
    }
}
