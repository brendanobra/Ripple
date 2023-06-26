use crate::client::xvp_session::XvpSession;

use dpab_core::{
    gateway::DpabDelegate,
    message::{DpabError, DpabRequest, DpabRequestPayload, DpabResponse, DpabResponsePayload},
    model::discovery::{
        ClearContentSetParams, ContentAccessListSetParams, ContentAccessResponse, DiscoveryRequest,
    },
};

use tokio::sync::oneshot::Sender as OneShotSender;
use tonic::async_trait;
use tracing::error;

type Callback = Option<OneShotSender<DpabResponse>>;
pub struct DiscoveryService {
    endpoint: String,
}

#[async_trait]
impl DpabDelegate for DiscoveryService {
    async fn handle(&mut self, request: DpabRequest) {
        if let DpabRequestPayload::Discovery(discovery_request) = request.payload {
            match discovery_request {
                DiscoveryRequest::SetContentAccess(params) => {
                    self.set_content_access(params, request.callback).await
                }
                DiscoveryRequest::ClearContent(params) => {
                    self.clear_content_access(params, request.callback).await
                }
            }
        } else {
            error!("handle: Unexpected payload: {:?}", request.payload);
        }
    }
}

impl DiscoveryService {
    pub fn new(endpoint: String) -> DiscoveryService {
        DiscoveryService { endpoint }
    }

    async fn set_content_access(&mut self, params: ContentAccessListSetParams, callback: Callback) {
        match XvpSession::set_content_access(self.endpoint.clone(), params).await {
            Ok(_) => {
                let resp =
                    DpabResponse::Ok(DpabResponsePayload::ContentAccess(ContentAccessResponse {}));
                if let Some(cb) = callback {
                    cb.send(resp).ok();
                }
            }
            Err(_) => {
                if let Some(cb) = callback {
                    cb.send(DpabResponse::Err(DpabError::ServiceError)).ok();
                }
            }
        }
    }

    async fn clear_content_access(&mut self, params: ClearContentSetParams, callback: Callback) {
        match XvpSession::clear_content_access(self.endpoint.clone(), params).await {
            Ok(_) => {
                let resp =
                    DpabResponse::Ok(DpabResponsePayload::ContentAccess(ContentAccessResponse {}));
                if let Some(cb) = callback {
                    cb.send(resp).ok();
                }
            }
            Err(_) => {
                if let Some(cb) = callback {
                    cb.send(DpabResponse::Err(DpabError::ServiceError)).ok();
                }
            }
        }
    }
}
