use std::sync::{Arc, Mutex};

use dpab_core::gateway::DpabDelegate;
use dpab_core::message::{DpabError, DpabRequest, DpabResponsePayload};

use dpab_core::model::auth::{AuthRequest, AuthService, GetPlatformTokenParams};

use async_trait::async_trait;
use dpab_core::model::thor_permission_registry::ThorPermissionRegistry;
use tracing::{error, info_span, trace, Instrument};

use crate::gateway::appsanity_gateway::GrpcClientSession;

use super::thor_permission::ThorPermissionService;

use std::convert::From;

pub struct AppsanityAuthService {
    thor_permission_service: Arc<ThorPermissionService>,
}

impl AppsanityAuthService {
    fn new(thor_permission_service: ThorPermissionService) -> Box<Self>
    where
        Self: AuthService<'static>,
    {
        Box::new(AppsanityAuthService {
            thor_permission_service: Arc::new(thor_permission_service),
        })
    }
}

#[async_trait]
impl AuthService<'_> for AppsanityAuthService {
    async fn get_platform_token(
        self: Box<Self>,
        request: DpabRequest,
        params: GetPlatformTokenParams,
    ) -> Result<String, DpabError> {
        let sess = params.dist_session;
        match self
            .thor_permission_service
            .clone()
            .get_thor_token(
                &sess,
                params.app_id,
                params.content_provider,
                params.device_session_id,
                params.app_session_id,
            )
            .await
        {
            Ok(token) => {
                let response = Ok(DpabResponsePayload::String(token.clone()));
                request.respond_and_log(response);
                Ok(token)
            }
            Err(thor_error) => {
                error!(
                    "Thor Permission Service returned an error, err={:?}",
                    thor_error
                );
                request.respond_and_log(Err(DpabError::ServiceError));
                Err(DpabError::ServiceError)
            }
        }
    }
}

pub struct AuthDelegate {
    pub permission_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
}

#[async_trait]
impl DpabDelegate for AuthDelegate {
    async fn handle(&mut self, request: DpabRequest) {
        let parent_span = match request.parent_span.clone() {
            Some(s) => s,
            None => info_span!("appsanity auth delegate handling request"),
        };
        let span = info_span!(
            parent: &parent_span,
            "appsanity auth delegate handling request",
            ?request
        );
        let tps = ThorPermissionService::new_from(
            self.permission_service_grpc_client_session.clone(),
            ThorPermissionRegistry::new(),
        );

        let service = AppsanityAuthService::new(tps);

        match request.payload.as_auth_request() {
            Some(req) => match req {
                AuthRequest::GetPlatformToken(get_platform_token_params) => {
                    trace!("appsanity.auth.GetPlatformToken");
                    let future = async move {
                        let _res = service
                            .get_platform_token(request, get_platform_token_params)
                            .await;
                    };
                    tokio::spawn(future.instrument(span));
                }
            },
            None => error!("{:?}", DpabError::ServiceError),
        }
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use crate::util::service_util::create_grpc_client_session;
    use dpab_core::model::api_grants::ApiName;
    use dpab_core::model::auth::AuthRequest;
    use dpab_core::model::auth::GetAppPermissionsParams;
    use dpab_core::model::auth::GetPlatformTokenParams;
    use dpab_core::{
        message::{DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse},
        model::auth,
    };
    use tokio::sync::{
        mpsc,
        oneshot::{self, Receiver, Sender},
    };

    use std::str::FromStr;
    use std::{collections::HashMap, fmt::Debug};
    use tonic::transport::ClientTlsConfig;
    use tracing::Level;

    use super::*;

    #[tokio::test]
    async fn test_handle_platform_token() {
        let mut passed = false;
        let (dpab_req_tx, dpab_req_rx) = mpsc::channel::<DpabRequest>(32);
        let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();

        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_auth"),
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
            let channel_url = format!("ad-platform-service.svc.thor.comcast.com");
            let permission_service_grpc_client_session = create_grpc_client_session(channel_url);

            let mut app_ignore_rules = HashMap::new();

            let mut method_ignore_rules = Vec::with_capacity(4);
            method_ignore_rules.push(String::from("GetAdIdObject"));
            method_ignore_rules.push(String::from("GetPlatformToken"));
            method_ignore_rules.push(String::from("GetAppMethodPermission"));

            app_ignore_rules.insert(String::from("App_1"), method_ignore_rules.clone());

            let mut auth_delegate = AuthDelegate {
                permission_service_grpc_client_session,
            };

            auth_delegate.handle(get_dpab_request).await;
        });

        if let Ok(x) = dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }
}
