use crate::gateway::appsanity_gateway::GrpcClientSession;
use crate::secure_storage_service::secure_storage_service_client::SecureStorageServiceClient;
use crate::secure_storage_service::{
    key, value::Expiration, DeleteValueRequest as GrpcDeleteValueRequest,
    GetValueRequest as GrpcGetValueRequest, Key, UpdateValueRequest as GrpcUpdateValueRequest,
    Value,
};
use crate::util::service_util::decorate_request_with_session;
use async_trait::async_trait;
use dpab_core::gateway::DpabDelegate;
use serde::{Deserialize, Serialize};

use dpab_core::message::{
    DpabError, DpabRequest, DpabRequestPayload, DpabResponse, DpabResponsePayload,
};
use dpab_core::model::secure_storage::{
    SecureStorageGetRequest, SecureStorageGetResponse, SecureStorageRemoveRequest,
    SecureStorageRemoveResponse, SecureStorageRequest, SecureStorageResponse,
    SecureStorageSetRequest, SecureStorageSetResponse, StorageSetOptions,
};
use prost_types::Duration;
use std::sync::{Arc, Mutex};
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};
use tracing::{error, info};

use std::convert::From;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LocalSecureStorageSetRequest(SecureStorageSetRequest);

impl From<LocalSecureStorageSetRequest> for Key {
    fn from(request: LocalSecureStorageSetRequest) -> Self {
        Key {
            app_id: request.0.app_id,
            key: request.0.key,
            scope: request.0.scope as i32,
        }
    }
}

impl From<LocalSecureStorageSetRequest> for Option<Expiration> {
    fn from(request: LocalSecureStorageSetRequest) -> Self {
        request.0.options.map(|sso| {
            Expiration::Ttl(Duration {
                seconds: sso.ttl as i64,
                nanos: 0,
            })
        })
    }
}

impl From<LocalSecureStorageSetRequest> for Value {
    fn from(request: LocalSecureStorageSetRequest) -> Self {
        Value {
            key: Some(request.clone().into()),
            value: request.clone().0.value,
            expiration: request.clone().into(),
        }
    }
}

impl From<SecureStorageRemoveRequest> for GrpcDeleteValueRequest {
    fn from(request: SecureStorageRemoveRequest) -> Self {
        GrpcDeleteValueRequest {
            key: Some(Key {
                app_id: request.app_id,
                key: request.key,
                scope: request.scope as i32,
            }),
        }
    }
}

impl From<SecureStorageGetRequest> for GrpcGetValueRequest {
    fn from(request: SecureStorageGetRequest) -> Self {
        GrpcGetValueRequest {
            key: Some(Key {
                app_id: request.app_id,
                key: request.key,
                scope: request.scope as i32,
            }),
        }
    }
}

impl From<SecureStorageSetRequest> for GrpcUpdateValueRequest {
    fn from(value: SecureStorageSetRequest) -> Self {
        let setReq = LocalSecureStorageSetRequest(value);
        GrpcUpdateValueRequest {
            allow_missing: true,
            value: Some(setReq.into()),
        }
    }
}

pub struct DistPSecureStorageService {
    pub distp_secure_storage_service_grpc_session: Arc<Mutex<GrpcClientSession>>,
}
impl DistPSecureStorageService {
    pub fn new(
        distp_secure_storage_service_grpc_session: Arc<Mutex<GrpcClientSession>>,
    ) -> DistPSecureStorageService {
        DistPSecureStorageService {
            distp_secure_storage_service_grpc_session: distp_secure_storage_service_grpc_session,
        }
    }
    fn get_channel(&self) -> Channel {
        self.distp_secure_storage_service_grpc_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone()
    }
    async fn process_request(&self, request: SecureStorageRequest) -> DpabResponse {
        match request {
            dpab_core::model::secure_storage::SecureStorageRequest::Get(get) => {
                let grpc_request: GrpcGetValueRequest = get.clone().into();
                let mut svc = SecureStorageServiceClient::with_interceptor(
                    self.get_channel().clone(),
                    |mut req: Request<()>| {
                        decorate_request_with_session(&mut req, &get.clone().distributor_session);
                        Ok(req)
                    },
                );
                match svc.get_value(grpc_request).await {
                    Ok(ok) => {
                        let value = match ok.into_inner().value {
                            Some(value) => Some(value.value),
                            _ => None,
                        };
                        DpabResponse::Ok(DpabResponsePayload::SecureStorage(
                            SecureStorageResponse::Get(SecureStorageGetResponse { value: value }),
                        ))
                    }
                    Err(_) => DpabResponse::Err(DpabError::ServiceError),
                }
            }
            dpab_core::model::secure_storage::SecureStorageRequest::Set(set) => {
                let grpc_request: GrpcUpdateValueRequest = set.clone().into();
                let mut svc = SecureStorageServiceClient::with_interceptor(
                    self.get_channel(),
                    |mut req: Request<()>| {
                        decorate_request_with_session(&mut req, &set.clone().distributor_session);
                        Ok(req)
                    },
                );
                match svc.update_value(grpc_request).await {
                    Ok(_) => DpabResponse::Ok(DpabResponsePayload::SecureStorage(
                        SecureStorageResponse::Set(SecureStorageSetResponse {}),
                    )),
                    Err(_) => DpabResponse::Err(DpabError::ServiceError),
                }
            }
            dpab_core::model::secure_storage::SecureStorageRequest::Remove(remove) => {
                let grpc_request: GrpcDeleteValueRequest = remove.clone().into();
                let mut svc = SecureStorageServiceClient::with_interceptor(
                    self.get_channel(),
                    |mut req: Request<()>| {
                        decorate_request_with_session(
                            &mut req,
                            &remove.clone().distributor_session,
                        );
                        Ok(req)
                    },
                );
                match svc.delete_value(grpc_request).await {
                    Ok(_) => DpabResponse::Ok(DpabResponsePayload::SecureStorage(
                        SecureStorageResponse::Remove(SecureStorageRemoveResponse {}),
                    )),
                    Err(_) => DpabResponse::Err(DpabError::ServiceError),
                }
            }
        }
    }
}

#[async_trait]
impl DpabDelegate for DistPSecureStorageService {
    async fn handle(&mut self, request: DpabRequest) {
        if let DpabRequestPayload::SecureStorage(storage_request) = request.payload.clone() {
            request.respond_and_log(
                Box::new(DistPSecureStorageService {
                    distp_secure_storage_service_grpc_session: self
                        .distp_secure_storage_service_grpc_session
                        .clone(),
                })
                .process_request(storage_request)
                .await,
            );
        } else {
            error!("handle: Unexpected payload: {:?}", request.payload);
        }
    }
}
