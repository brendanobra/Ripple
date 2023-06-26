use std::sync::{Arc, Mutex};

use crate::{
    ad_platform::{
        ad_platform_service_client::AdPlatformServiceClient, AdInitObjectRequest, ResetXifaRequest,
        XifaRequest,
    },
    gateway::appsanity_gateway::GrpcClientSession,
    util::service_util::decorate_request_with_session,
};
use async_trait::async_trait;
use dpab_core::{
    gateway::DpabDelegate,
    message::{DistributorSession, DpabError, DpabRequest, DpabResponsePayload},
    model::advertising::{
        AdIdRequestParams, AdIdResponse, AdInitObjectRequestParams, AdInitObjectResponse,
        AdvertisingRequest, AdvertisingService,
    },
};

use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};
use tracing::{debug, error, info_span, Instrument};

pub struct AppsanityAdvertisingService {
    grpc_client_session: Arc<Mutex<GrpcClientSession>>,
}

#[async_trait]
impl AdvertisingService for AppsanityAdvertisingService {
    async fn get_ad_init_object(
        self: Box<Self>,
        request: DpabRequest,
        params: AdInitObjectRequestParams,
    ) {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();

        let mut client =
            AdPlatformServiceClient::with_interceptor(channel, |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &params.dist_session);
                Ok(req)
            });
        let perm_req = tonic::Request::new(AdInitObjectRequest {
            privacy_data: params.privacy_data,
            durable_app_id: params.durable_app_id,
            app_version: params.app_version,
            distributor_app_id: params.distributor_app_id,
            device_ad_attributes: params.device_ad_attributes,
            coppa: params.coppa,
            authentication_entity: params.authentication_entity,
            environment: params.environment,
        });
        match client.get_ad_init_object(perm_req).await {
            Ok(res) => {
                let res_i = res.into_inner();
                let response = AdInitObjectResponse {
                    ad_server_url: res_i.ad_server_url,
                    ad_server_url_template: res_i.ad_server_url_template,
                    ad_network_id: res_i.ad_network_i_d,
                    ad_profile_id: res_i.ad_profile_i_d,
                    ad_site_section_id: res_i.ad_site_section_i_d,
                    ad_opt_out: res_i.ad_opt_out,
                    privacy_data: res_i.privacy_data,
                    ifa_value: res_i.ifa_value,
                    ifa: res_i.ifa,
                    app_name: res_i.app_name,
                    app_bundle_id: res_i.app_bundle_i_d,
                    app_version: res_i.app_version,
                    distributor_app_id: res_i.distributor_app_i_d,
                    device_ad_attributes: res_i.device_ad_attributes,
                    coppa: res_i.coppa,
                    authentication_entity: res_i.authentication_entity,
                };
                request.respond_and_log(Ok(DpabResponsePayload::AdInitObject(response)));
            }
            Err(e) => {
                error!("Error getting ad init object: {:?}", e);
                request.respond_and_log(Err(DpabError::IoError));
            }
        }
    }

    async fn get_ad_identifier(self: Box<Self>, request: DpabRequest, params: AdIdRequestParams) {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();

        let mut client =
            AdPlatformServiceClient::with_interceptor(channel, |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &params.dist_session);
                Ok(req)
            });
        let perm_req = tonic::Request::new(XifaRequest {
            privacy_data: params.privacy_data,
            durable_app_id: params.app_id,
        });
        match client.get_xifa(perm_req).await {
            Ok(res) => {
                let ifa_i = res.into_inner().ifa.unwrap();
                let response = AdIdResponse {
                    ifa: ifa_i.ifa,
                    ifa_type: ifa_i.ifa_type,
                    lmt: ifa_i.lmt,
                };
                request.respond_and_log(Ok(DpabResponsePayload::AdIdObject(response)));
            }
            Err(e) => {
                error!("Error getting xifa: {:?}", e);
                request.respond_and_log(Err(DpabError::IoError));
            }
        }
    }

    async fn reset_ad_identifier(
        self: Box<Self>,
        request: DpabRequest,
        params: DistributorSession,
    ) {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();
        let mut client =
            AdPlatformServiceClient::with_interceptor(channel, |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &params);
                Ok(req)
            });
        let perm_req = tonic::Request::new(ResetXifaRequest {});
        match client.reset_xifa(perm_req).await {
            Ok(res) => {
                debug!("XIFA reset successful received response {:?}", res);
                request.respond_and_log(Ok(DpabResponsePayload::None));
            }
            Err(e) => {
                error!("Error resetting xifa: {:?}", e);
                request.respond_and_log(Err(DpabError::IoError));
            }
        }
    }
}

pub struct AdvertisingDelegate {
    pub ad_platform_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
}

#[async_trait]
impl DpabDelegate for AdvertisingDelegate {
    async fn handle(&mut self, request: DpabRequest) {
        let parent_span = match request.parent_span.clone() {
            Some(s) => s,
            None => info_span!("appsanity advertising delegate handling request"),
        };
        let span = info_span!(
            parent: &parent_span,
            "appsanity advertising delegate handling request",
            ?request
        );
        let service = Box::new(AppsanityAdvertisingService {
            grpc_client_session: self.ad_platform_service_grpc_client_session.clone(),
        });
        match request.payload.as_advertising_request() {
            Some(req) => match req {
                AdvertisingRequest::GetAdInitObject(get_ad_init_object_params) => {
                    let future = async move {
                        service
                            .get_ad_init_object(request, get_ad_init_object_params)
                            .await;
                    };
                    tokio::spawn(future.instrument(span));
                }

                AdvertisingRequest::GetAdIdObject(session) => {
                    let future = async move {
                        service.get_ad_identifier(request, session).await;
                    };
                    tokio::spawn(future.instrument(span));
                }
                AdvertisingRequest::ResetAdIdentifier(session) => {
                    let future = async move {
                        service.reset_ad_identifier(request, session).await;
                    };
                    tokio::spawn(future.instrument(span));
                }
            },
            None => error!("Invalid dpab payload for Advertising module"),
        }
    }
}

#[allow(unused)]
#[cfg(test)]
pub mod tests {
    use dpab_core::gateway::DpabDelegate;
    use dpab_core::message::{DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse};
    use dpab_core::model::advertising::AdInitObjectRequestParams;
    use dpab_core::model::advertising::{AdIdRequestParams, AdvertisingRequest, SessionParams};
    use tokio::sync::mpsc;

    use crate::service::appsanity_advertising::AdvertisingDelegate;
    use crate::service::appsanity_resolver::AppsanityDelegate;

    use tokio::sync::oneshot::{self, Receiver, Sender};

    use crate::util::service_util::create_grpc_client_session;
    use std::collections::HashMap;
    use std::str::FromStr;

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

    #[tokio::test]
    pub async fn test_get_ad_init_object() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_1"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let mut privacy_dt = HashMap::new();
        privacy_dt.insert(String::from("app_id"), String::from("device_id"));

        let mut device_ad_att = HashMap::new();
        device_ad_att.insert(String::from("device_ad"), String::from("device_id"));

        let ad_init_object_request_params = AdInitObjectRequestParams {
            privacy_data: privacy_dt,
            environment: String::from("environmet"),
            durable_app_id: String::from("durable_app_id"),
            app_version: String::from("app_ver_1"),
            distributor_app_id: String::from("dist_app_id"),
            device_ad_attributes: device_ad_att,
            coppa: true,
            authentication_entity: String::from("auth_entity"),
            dist_session: get_distributer_session.clone(),
        };

        let get_dpab_request = DpabRequest {
            payload: DpabRequestPayload::Advertising(AdvertisingRequest::GetAdInitObject(
                ad_init_object_request_params.clone(),
            )),
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };
        let test_grpc_client_session =
            create_grpc_client_session(String::from("ad-platform-service.svc-qa.thor.comcast.com"));
        let mut advertising_delegate = AdvertisingDelegate {
            ad_platform_service_grpc_client_session: test_grpc_client_session,
        };
        advertising_delegate.handle(get_dpab_request).await;

        if let Ok(x) = dpab_test.dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }

    #[tokio::test]
    pub async fn test_get_ad_id_object() {
        let mut passed = false;
        let dpab_test = DpabTest::new();
        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_adveristing"),
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
            callback: Some(dpab_test.dpab_res_tx),
            parent_span: None,
        };
        let test_grpc_client_session =
            create_grpc_client_session(String::from("ad-platform-service.svc-qa.thor.comcast.com"));

        let mut advertising_delegate = AdvertisingDelegate {
            ad_platform_service_grpc_client_session: test_grpc_client_session,
        };
        advertising_delegate.handle(get_dpab_request).await;

        if let Ok(x) = dpab_test.dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }
}
