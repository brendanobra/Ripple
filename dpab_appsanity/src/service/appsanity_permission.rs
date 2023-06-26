use std::sync::{Arc, Mutex};

use crate::gateway::appsanity_gateway::GrpcClientSession;
use crate::util::channel_util::oneshot_send_and_log;

use super::thor_permission::ThorPermissionService;
use dpab_core::model::thor_permission_registry::ThorPermissionRegistry;
use dpab_core::{
    gateway::DpabDelegate,
    message::{DpabError, DpabRequest, DpabResponsePayload, PermissionServiceError},
    model::permissions::PermissionService,
};
use tonic::async_trait;
use tracing::{error, info_span, Instrument};

pub struct PermissionDelegate {
    pub permission_service_grpc_client_session: Arc<Mutex<GrpcClientSession>>,
}

#[async_trait]
impl DpabDelegate for PermissionDelegate {
    async fn handle(&mut self, request: DpabRequest) {
        let ps = request.parent_span.clone();
        let payload = request.payload.as_permission_request();
        let parent_span = match ps {
            Some(s) => s,
            None => info_span!("appsanity permission delegate handling request"),
        };
        let span = info_span!(
            parent: &parent_span,
            "appsanity permission delegate handling request",
            ?request
        );

        if let Some(callback) = request.callback {
            if let Some(r) = payload {
                let tps = Box::new(ThorPermissionService::new_from(
                    self.permission_service_grpc_client_session.clone(),
                    ThorPermissionRegistry::new(),
                ));

                oneshot_send_and_log(
                    callback,
                    match tps.handle_permission(r).instrument(span).await {
                        Ok(p) => Ok(DpabResponsePayload::Permission(p)),
                        Err(e) => Err(e),
                    },
                    "appsanity_permission_check",
                );
            } else {
                oneshot_send_and_log(
                    callback,
                    Err(DpabError::PermissionServiceError(PermissionServiceError {
                        provider: "appsanity".to_string(),
                        message: format!("{:?}", DpabError::ServiceError),
                    })),
                    "dpab_permission",
                );
            }
        } else {
            error!("{:?}", DpabError::ServiceError)
        }
    }
}
