use dpab_core::message::DpabResponsePayload;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use crate::{
    service::appsanity_privacy::PrivacyService,
    util::cloud_sync_monitor_utils::SyncAndMonitorProcessor,
};
pub struct UserGrantsSyncMonitorService {
    supported_properties: Vec<String>,
}
impl UserGrantsSyncMonitorService {
    pub fn new(cloud_firebolt_mapping: &Value) -> Self {
        UserGrantsSyncMonitorService {
            supported_properties: PrivacyService::get_user_grants_mapping(cloud_firebolt_mapping)
                .keys()
                .cloned()
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl SyncAndMonitorProcessor for UserGrantsSyncMonitorService {
    fn get_properties(&self) -> Vec<String> {
        self.supported_properties.clone()
    }

    async fn process_and_send_response(
        &self,
        callback: Sender<DpabResponsePayload>,
        payload: DpabResponsePayload,
    ) {
        if let DpabResponsePayload::UserGrants(privacy_response) = payload {
            let _ = callback
                .send(DpabResponsePayload::UserGrants(privacy_response))
                .await;
        }
    }
}
