use std::sync::{Arc, Mutex};

use dpab_core::message::DistributorSession;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};

use crate::gateway::appsanity_gateway::GrpcClientSession;

pub fn decorate_request_with_session(req: &mut Request<()>, session: &DistributorSession) {
    let bearer = format!("Bearer {}", session.token);
    req.metadata_mut()
        .insert("authorization", (bearer.as_str().parse()).unwrap());
    req.metadata_mut()
        .insert("deviceid", (session.device_id.as_str().parse()).unwrap());
    req.metadata_mut()
        .insert("accountid", (session.account_id.as_str().parse()).unwrap());
    req.metadata_mut()
        .insert("partnerid", (session.id.as_str().parse()).unwrap());
}
pub fn create_lazy_tls_channel(service_url: String) -> Channel {
    tonic::transport::Channel::from_shared(format!("https://{}", service_url.clone()))
        .unwrap()
        .tls_config(ClientTlsConfig::new().domain_name(service_url.clone()))
        .unwrap()
        .connect_lazy()
}

pub fn create_grpc_client_session(service_url: String) -> Arc<Mutex<GrpcClientSession>> {
    let endpoint =
        tonic::transport::Channel::from_shared(format!("https://{}", service_url.clone()))
            .unwrap()
            .tls_config(ClientTlsConfig::new().domain_name(service_url.clone()))
            .unwrap();
    Arc::new(Mutex::new(GrpcClientSession::new(endpoint)))
}

/*
non tls version for testing with locally running microservices */
pub fn create_lazy_insecure_channel(service_url: String) -> Channel {
    tonic::transport::Channel::from_shared(format!("http://{}", service_url.clone()))
        .unwrap()
        .connect_lazy()
}
