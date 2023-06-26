use crate::message::{DistributorSession, DpabResponsePayload};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum SyncAndMonitorRequest {
    SyncAndMonitor(
        SyncAndMonitorModule,
        DistributorSession,
        Sender<DpabResponsePayload>,
    ),
    UpdateDistributorToken(String),
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SyncAndMonitorModule {
    Privacy,
    UserGrants,
}
