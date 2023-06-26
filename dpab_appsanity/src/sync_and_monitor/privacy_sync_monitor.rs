// This is a sample implementation. This should be replaced with the actual implementation

use crate::util::cloud_sync_monitor_utils::SyncAndMonitorProcessor;
use async_trait::async_trait;
use dpab_core::{message::DpabResponsePayload, model::privacy::PrivacyResponse};
use tokio::sync::mpsc::Sender;
pub struct PrivacySyncMonitorService;
impl PrivacySyncMonitorService {
    pub fn new() -> Self {
        PrivacySyncMonitorService {}
    }
}
#[async_trait]
impl SyncAndMonitorProcessor for PrivacySyncMonitorService {
    fn get_properties(&self) -> Vec<String> {
        let supported_properties = vec![
            "xcal:continueWatching".to_string(),
            "xcal:unentitledContinueWatching".to_string(),
            "xcal:watchHistory".to_string(),
            "xcal:productAnalytics".to_string(),
            "xcal:personalization".to_string(),
            "xcal:unentitledPersonalization".to_string(),
            "xcal:remoteDiagnostics".to_string(),
            "xcal:primaryContentAdTargeting".to_string(),
            "xcal:primaryBrowseAdTargeting".to_string(),
            "xcal:appContentAdTargeting".to_string(),
            "xcal:acr".to_string(),
            "xcal:cameraAnalytics".to_string(),
        ];
        supported_properties
    }

    async fn process_and_send_response(
        &self,
        callback: Sender<DpabResponsePayload>,
        payload: DpabResponsePayload,
    ) {
        if let DpabResponsePayload::Privacy(privacy_response) = payload {
            if let PrivacyResponse::Settings(settings) = privacy_response {
                let mut settings_c = settings.clone();
                // Making app data collection and entitlement collection as none as these
                // pretain to user grants and not privacy settings.
                settings_c.app_data_collection = None;
                settings_c.app_entitlement_collection = None;
                let _ = callback.send(DpabResponsePayload::Privacy(PrivacyResponse::Settings(
                    settings_c,
                )));
            }
        }
    }
}
