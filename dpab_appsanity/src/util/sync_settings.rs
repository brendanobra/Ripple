use dpab_core::{
    message::{DpabError, DpabRequest, DpabRequestPayload, DpabResponsePayload},
    model::{privacy::PrivacyRequest, sync_and_monitor::SyncAndMonitorModule},
};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::debug;

use crate::service::appsanity_privacy::PrivacyService;
use dpab_core::{gateway::DpabDelegate, message::DistributorSession};

use super::cloud_sync_monitor_utils::StateRequest;
#[derive(Debug, Clone)]
pub struct SyncSettings {
    pub module: SyncAndMonitorModule,
    pub dist_session: DistributorSession,
    pub cloud_service_url: String,
    pub cloud_sync_ttl: u32,
    pub cloud_monitor_topic: String,
    pub settings: Vec<String>,
    pub callback: Sender<DpabResponsePayload>,
    pub cloud_firebolt_mapping: Value,
}

impl Eq for SyncSettings {}

impl PartialEq for SyncSettings {
    fn eq(&self, other: &SyncSettings) -> bool {
        self.cloud_monitor_topic == other.cloud_monitor_topic && self.settings.eq(&other.settings)
    }
}

impl Hash for SyncSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.settings.hash(state);
        self.cloud_monitor_topic.hash(state);
    }
}

impl SyncSettings {
    pub async fn get_values_from_cloud(
        &self,
        state_tx: Sender<StateRequest>,
    ) -> Result<DpabResponsePayload, DpabError> {
        let (tx, rx) = oneshot::channel();
        let _ = state_tx.send(StateRequest::GetDistributorToken(tx)).await;
        let mut dist_session = self.dist_session.clone();
        let res_token = rx.await;
        if res_token.is_err() {
            return Err(DpabError::ServiceError);
        }
        let token = res_token.unwrap();
        dist_session.token = token.to_owned();
        match self.module {
            SyncAndMonitorModule::Privacy => {
                let mut privacy_service = PrivacyService::new(
                    self.cloud_service_url.clone(),
                    dist_session.clone(),
                    &self.cloud_firebolt_mapping,
                );
                let result = privacy_service.get_properties(dist_session.clone()).await;
                result
            }
            SyncAndMonitorModule::UserGrants => {
                let mut privacy_service = PrivacyService::new(
                    self.cloud_service_url.clone(),
                    dist_session.clone(),
                    &self.cloud_firebolt_mapping,
                );
                let result = privacy_service.get_user_grants().await;
                result
            }
        }
    }
}
