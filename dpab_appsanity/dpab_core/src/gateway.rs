use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::message::{DistributorSession, DpabRequest};

#[derive(Debug)]
pub struct GatewayContext {
    pub session: Option<DistributorSession>,
    pub sender: Sender<DpabRequest>,
    pub receiver: Receiver<DpabRequest>,
    pub config: Option<Value>,
}

#[async_trait]
pub trait Gateway {
    async fn start(self: Box<Self>, gateway_context: GatewayContext) -> Box<Self>;
    async fn shutdown(self: Box<Self>) -> bool;
}
#[async_trait]
pub trait DpabDelegate: Send + Sync {
    async fn handle(&mut self, request: DpabRequest);
}
