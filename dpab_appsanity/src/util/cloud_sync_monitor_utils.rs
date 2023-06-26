use async_trait::async_trait;
use dpab_core::{
    message::{DistributorSession, DpabResponsePayload},
    model::sync_and_monitor::SyncAndMonitorModule,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    gateway::appsanity_gateway::AppsanityConfig,
    sync_and_monitor::{
        privacy_sync_monitor::PrivacySyncMonitorService,
        user_grants_sync_monitor::UserGrantsSyncMonitorService,
    },
};

use super::sync_settings::SyncSettings;
#[derive(Debug)]
pub enum LinchpinProxyCommand {
    Connect(String, String, String), // linchpin_url, device_id, SAT
    Subscribe(String),               // Topic to Subscribe
    Unsubscribe(String),             //Topic to Unsubscribe
    UpdateDistributorToken(String),
}

#[derive(Debug)]
pub enum ConvertError {
    NoKeyPresent,
    KeyHasNoValue,
    GenericError,
}

#[derive(Debug)]
pub enum StateRequest {
    AddListener(SyncSettings),
    RemoveListener(SyncSettings),
    AddPendingTopic(String),
    SetLinchpinConnectionStatus(bool),
    GetLinchpinConnectionStatus(oneshot::Sender<bool>),
    GetListeningTopics(oneshot::Sender<Vec<String>>),
    GetListenersForProperties(String, Vec<String>, oneshot::Sender<Vec<SyncSettings>>), //topic, property, cb
    GetListenersForModule(
        String,
        SyncAndMonitorModule,
        oneshot::Sender<Vec<SyncSettings>>,
    ),
    GetAllPendingTopics(oneshot::Sender<Vec<String>>),
    SetDistributorToken(String),
    GetDistributorToken(oneshot::Sender<String>),
    ClearPendingTopics,
}

#[derive(Debug, Deserialize)]
pub struct EventPayload {
    environment: String,
    pub settings: Value,
}

#[derive(Debug, Deserialize)]
pub struct LinchpinPayload {
    pub event_payload: EventPayload,
    timestamp: u64,
    event_schema: String,
    event_id: String,
    account_id: String,
    partner_id: String,
    source: String,
}

#[async_trait]
pub trait SyncAndMonitorProcessor {
    fn get_properties(&self) -> Vec<String>;
    async fn process_and_send_response(
        &self,
        callback: Sender<DpabResponsePayload>,
        response: DpabResponsePayload,
    );
}

pub fn replace_uri_variables(base: &str, dist_session: &DistributorSession) -> String {
    let mut new_str = base.to_owned();
    new_str = new_str.replace("{partnerId}", &dist_session.id);
    new_str = new_str.replace("{accountId}", &dist_session.account_id);
    new_str = new_str.replace("{clientId}", "ripple");
    new_str
}

pub fn get_request_processor(
    module: SyncAndMonitorModule,
    appsanity_config: &AppsanityConfig,
) -> Box<dyn SyncAndMonitorProcessor> {
    match module {
        SyncAndMonitorModule::Privacy => Box::new(PrivacySyncMonitorService::new()),
        SyncAndMonitorModule::UserGrants => Box::new(UserGrantsSyncMonitorService::new(
            &appsanity_config.cloud_firebolt_mapping,
        )),
    }
}

pub fn get_sync_settings(
    module: SyncAndMonitorModule,
    appsanity_config: AppsanityConfig,
    dist_session: DistributorSession,
    callback: Sender<DpabResponsePayload>,
) -> SyncSettings {
    let request_handler: Box<dyn SyncAndMonitorProcessor> =
        get_request_processor(module, &appsanity_config);
    let mut sync_settings = SyncSettings {
        module,
        dist_session,
        cloud_service_url: Default::default(),
        cloud_sync_ttl: Default::default(),
        cloud_monitor_topic: Default::default(),
        settings: Default::default(),
        cloud_firebolt_mapping: appsanity_config.cloud_firebolt_mapping.clone(),
        callback,
    };
    match module {
        SyncAndMonitorModule::UserGrants | SyncAndMonitorModule::Privacy => {
            sync_settings.cloud_service_url = replace_uri_variables(
                &appsanity_config.privacy_service.url,
                &sync_settings.dist_session,
            );
            sync_settings.cloud_monitor_topic = replace_uri_variables(
                &appsanity_config
                    .get_linchpin_topic_for_url(&appsanity_config.privacy_service.url)
                    .unwrap_or_default(),
                &sync_settings.dist_session,
            );
            sync_settings.cloud_sync_ttl = appsanity_config
                .get_ttl_for_url(&appsanity_config.privacy_service.url)
                .unwrap_or_default();
            sync_settings.settings = request_handler.get_properties().clone();
        }
    }
    sync_settings
}
