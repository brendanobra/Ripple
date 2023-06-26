use async_trait::async_trait;
use dpab_core::{
    gateway::{Gateway, GatewayContext},
    message::DpabRequest,
    model::metrics::{AppBehavioralMetric, BadgerMetrics},
};
use queues::CircularBuffer;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    iter::Map,
    sync::{Arc, Mutex},
    time::{self, Duration, SystemTime},
};
use tokio::sync::{
    mpsc::{self, Receiver},
    Mutex as TokioMutex, RwLock,
};
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error};

use crate::{
    ad_platform::ad_platform_service_server::AdPlatformService,
    service::appsanity_resolver::AppsanityServiceResolver,
    service::{
        appsanity_account_link::AppsanityAccountLinkService,
        appsanity_advertising::AppsanityAdvertisingService, appsanity_metrics::send_metrics,
        thor_permission::ThorPermissionService,
    },
    util::{
        cloud_linchpin_monitor::CloudLinchpinMonitor, cloud_periodic_sync::CloudPeriodicSync,
        cloud_sync_monitor_utils::StateRequest, service_util::create_grpc_client_session,
        sync_settings::SyncSettings,
    },
};

use crate::service::catalog::appsanity_catalog::CatalogServiceConfig;

#[derive(Deserialize, Debug, Clone)]
pub struct CloudService {
    pub url: String,
}
#[derive(Deserialize, Debug, Clone)]
pub struct CloudServiceScopes {
    content_access: String,
    sign_in_state: String,
}
impl CloudServiceScopes {
    pub fn get_xvp_content_access_scope(&self) -> &str {
        &self.content_access
    }
    pub fn get_xvp_sign_in_state_scope(&self) -> &str {
        &self.sign_in_state
    }
}

impl Default for CloudServiceScopes {
    fn default() -> Self {
        CloudServiceScopes {
            content_access: "account".to_string(),
            sign_in_state: "account".to_string(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct GrpcClientSession {
    pub endpoint: Endpoint,
    pub channel: Option<Channel>,
    last_accessed_time: Duration,
}

impl GrpcClientSession {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            channel: None,
            last_accessed_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        }
    }

    pub fn get_grpc_channel(&mut self) -> Channel {
        /* tonic does not have a good API to get the connection status or
        any connection related event notification. The available option is to configure the
        channel as keep-alive.  As a work around we are keeping the lazy connecton idle
        only for 4 minutes. The API call issued after 4 minutes of idle time, new channel will be
        created for communicating with the server */
        if let Some(channel) = &self.channel {
            let time_since_accessed = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .checked_sub(self.last_accessed_time)
                .unwrap_or(Duration::from_secs(0));

            if time_since_accessed < Duration::from_secs(240) {
                self.last_accessed_time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                return channel.clone();
            }
        }
        let lazy_channel = self.endpoint.connect_lazy();
        self.channel = Some(lazy_channel.clone());
        self.last_accessed_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        lazy_channel
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppAuthorizationRules {
    pub app_ignore_rules: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSchema {
    pub event_name: String,
    pub alias: Option<String>,
    pub namespace: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSchemas {
    pub default_metrics_namespace: String,
    pub default_metrics_schema_version: String,
    pub metrics_schemas: Vec<MetricsSchema>,
}
impl MetricsSchemas {
    pub fn get_event_name_alias(&self, event_name: &str) -> String {
        match self
            .metrics_schemas
            .iter()
            .find(|key| key.event_name == event_name)
        {
            Some(event) => {
                let default_name = &event_name.to_string();
                event
                    .alias
                    .as_ref()
                    .unwrap_or_else(|| default_name)
                    .to_string()
            }
            None => event_name.to_string(),
        }
    }

    pub fn get_event_path(&self, event_name: &str) -> String {
        match self
            .metrics_schemas
            .iter()
            .find(|key| key.event_name == event_name)
        {
            Some(metric) => {
                format!(
                    "{}/{}/{}",
                    metric
                        .namespace
                        .as_ref()
                        .unwrap_or_else(|| &self.default_metrics_namespace),
                    event_name,
                    metric
                        .version
                        .as_ref()
                        .unwrap_or_else(|| &self.default_metrics_schema_version)
                )
            }
            None => {
                format!(
                    "{}/{}/{}",
                    self.default_metrics_namespace.as_str(),
                    event_name,
                    self.default_metrics_schema_version.as_str()
                )
            }
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct SiftConfig {
    pub endpoint: String,
    pub api_key: String,
    pub batch_size: u8,
    pub max_queue_size: u8,
    pub send_interval_seconds: u16,
    #[serde(default = "metrics_schemas_default")]
    pub metrics_schemas: MetricsSchemas,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BehavioralMetricsConfig {
    pub sift: SiftConfig,
}

pub fn defaults() -> AppsanityConfig {
    /*
    build time, assume infallible and unwrap
    */
    /*
    TODO
    load defaults based on build conditions (i.e. debug, VBN, etc) or from filesystem?
    */
    serde_json::from_str::<AppsanityConfig>(include_str!("default_appsanity_config.json")).unwrap()
}

pub fn permission_service_default() -> CloudService {
    defaults().permission_service
}
pub fn ad_platform_service_default() -> CloudService {
    defaults().ad_platform_service
}
pub fn secure_storage_service_default() -> CloudService {
    defaults().secure_storage_service
}
pub fn xvp_playback_service_default() -> CloudService {
    defaults().xvp_playback_service
}
pub fn session_service_default() -> CloudService {
    defaults().session_service
}
pub fn app_authorization_rules_default() -> AppAuthorizationRules {
    defaults().app_authorization_rules
}
pub fn method_ignore_rules_default() -> Vec<String> {
    defaults().method_ignore_rules
}
pub fn behavioral_metrics_default() -> BehavioralMetricsConfig {
    defaults().behavioral_metrics
}
pub fn metrics_schemas_default() -> MetricsSchemas {
    defaults().behavioral_metrics.sift.metrics_schemas
}
pub fn discovery_service_default() -> CloudService {
    defaults().xvp_session_service
}
pub fn privacy_service_default() -> CloudService {
    defaults().privacy_service
}
pub fn xvp_video_service_default() -> CloudService {
    defaults().xvp_video_service
}
pub fn linchpin_service_default() -> SyncMonitorConfig {
    defaults().sync_monitor_service
}

pub fn cloud_firebolt_mapping_default() -> Value {
    defaults().cloud_firebolt_mapping
}
pub fn xvp_data_scopes_default() -> CloudServiceScopes {
    defaults().xvp_data_scopes
}
pub fn catalog_service_default() -> CatalogServiceConfig {
    defaults().catalog_service
}

#[derive(Deserialize, Debug, Clone)]
pub struct LinchpinServices {
    pub service_name: String,
    pub listen_topic: String,
    pub ttl: u32,
}
#[derive(Deserialize, Debug, Clone)]
pub struct SyncMonitorConfig {
    pub linchpin_url: String,
    pub services: Vec<LinchpinServices>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppsanityConfig {
    #[serde(default = "permission_service_default")]
    pub permission_service: CloudService,
    #[serde(default = "session_service_default")]
    pub session_service: CloudService,
    #[serde(default = "ad_platform_service_default")]
    pub ad_platform_service: CloudService,
    #[serde(default = "secure_storage_service_default")]
    pub secure_storage_service: CloudService,
    #[serde(default = "xvp_playback_service_default")]
    pub xvp_playback_service: CloudService,
    #[serde(default = "app_authorization_rules_default")]
    pub app_authorization_rules: AppAuthorizationRules,
    #[serde(default = "method_ignore_rules_default")]
    pub method_ignore_rules: Vec<String>,
    #[serde(default = "behavioral_metrics_default")]
    pub behavioral_metrics: BehavioralMetricsConfig,
    #[serde(default = "discovery_service_default")]
    pub xvp_session_service: CloudService,
    #[serde(default = "privacy_service_default")]
    pub privacy_service: CloudService,
    #[serde(default = "xvp_video_service_default")]
    pub xvp_video_service: CloudService,
    #[serde(default = "linchpin_service_default")]
    pub sync_monitor_service: SyncMonitorConfig,
    #[serde(default = "cloud_firebolt_mapping_default")]
    pub cloud_firebolt_mapping: Value,
    #[serde(default = "xvp_data_scopes_default")]
    pub xvp_data_scopes: CloudServiceScopes,
    #[serde(default = "catalog_service_default")]
    pub catalog_service: CatalogServiceConfig,
}

impl AppsanityConfig {
    pub fn get_service_name(&self, url: &str) -> Option<String> {
        if self.permission_service.url == url {
            return Some("permission_service".to_owned());
        }
        if self.session_service.url == url {
            return Some("session_service".to_owned());
        }
        if self.ad_platform_service.url == url {
            return Some("ad_platform_service".to_owned());
        }
        if self.secure_storage_service.url == url {
            return Some("secure_storage_service".to_owned());
        }
        if self.xvp_playback_service.url == url {
            return Some("xvp_playback_service".to_owned());
        }
        if self.xvp_session_service.url == url {
            return Some("xvp_session_service".to_owned());
        }
        if self.privacy_service.url == url {
            return Some("privacy_service".to_owned());
        }
        if self.xvp_video_service.url == url {
            return Some("xvp_video_service".to_owned());
        }
        None
    }
    pub fn get_linchpin_topic_for_service(&self, service: &str) -> Option<String> {
        let result = self
            .sync_monitor_service
            .services
            .iter()
            .find(|elem| elem.service_name == service.to_owned());
        if result.is_none() {
            return None;
        } else {
            return Some(result.unwrap().listen_topic.to_owned());
        }
    }
    pub fn get_ttl_for_service(&self, service: &str) -> Option<u32> {
        let result = self
            .sync_monitor_service
            .services
            .iter()
            .find(|elem| elem.service_name == service.to_owned());
        if result.is_none() {
            return None;
        } else {
            return Some(result.unwrap().ttl);
        }
    }
    pub fn get_linchpin_topic_for_url(&self, url: &str) -> Option<String> {
        let service_name = self.get_service_name(url)?;
        self.get_linchpin_topic_for_service(&service_name)
    }
    pub fn get_ttl_for_url(&self, url: &str) -> Option<u32> {
        let service_name = self.get_service_name(url)?;
        self.get_ttl_for_service(&service_name)
    }
}

#[derive(Debug)]
pub struct AppsanityGateway {}
fn get_config(maybe_value: &Option<Value>) -> AppsanityConfig {
    debug!("Passed on config value from ripple {:?}", maybe_value);
    match maybe_value {
        Some(config) => match serde_json::from_value::<AppsanityConfig>(config.to_owned()) {
            Ok(ok) => ok,
            Err(config_error) => {
                error!(
                    "dpab_appsanity config failed with {}, default values will be used",
                    config_error
                );
                defaults()
            }
        },
        None => {
            //Do defaults for completely empty config here
            defaults()
        }
    }
}
impl AppsanityGateway {
    pub async fn process_state_requests(mut rx: Receiver<StateRequest>) {
        debug!("Starting process state requests thread");
        let mut listener_map: HashMap<String, HashSet<SyncSettings>> = HashMap::new();
        let mut pending_topics: Vec<String> = Vec::new();
        let mut linchpin_connected = false;
        let mut sat_token = "".to_owned();
        while let Some(request) = rx.recv().await {
            match request {
                StateRequest::AddListener(listener) => {
                    let topic = listener.cloud_monitor_topic.to_owned();
                    debug!("Adding listener {:?} to topic: {}", listener, topic);
                    if let Some(listener_set) = listener_map.get_mut(topic.as_str()) {
                        listener_set.insert(listener);
                    } else {
                        listener_map.insert(topic, vec![listener].into_iter().collect());
                    }
                }
                StateRequest::RemoveListener(listener) => {
                    let topic = listener.cloud_monitor_topic.to_owned();
                    if let Some(listener_set) = listener_map.get_mut(topic.as_str()) {
                        listener_set.remove(&listener);
                    }
                }
                StateRequest::AddPendingTopic(topic) => {
                    pending_topics.push(topic);
                }
                StateRequest::GetListenersForProperties(topic, property_list, callback) => {
                    debug!(
                        "Getting listener for topic: {} and property: {:?}",
                        topic, property_list
                    );
                    let mut listener_list: Vec<SyncSettings> = vec![];
                    if let Some(listener_set) = listener_map.get(topic.as_str()) {
                        for listener in listener_set {
                            if property_list
                                .iter()
                                .all(|elem| listener.settings.contains(elem))
                            {
                                listener_list.push(listener.to_owned());
                            }
                        }
                    }
                    debug!("for properties listener list: {:?}", listener_list);
                    let _ = callback.send(listener_list);
                }
                StateRequest::GetAllPendingTopics(callback) => {
                    let _ = callback.send(pending_topics.clone());
                }
                StateRequest::ClearPendingTopics => {
                    pending_topics.clear();
                }
                StateRequest::GetListeningTopics(callback) => {
                    let _res = callback.send(listener_map.keys().cloned().collect());
                }
                StateRequest::SetLinchpinConnectionStatus(connected) => {
                    linchpin_connected = connected
                }
                StateRequest::GetLinchpinConnectionStatus(callback) => {
                    let _res = callback.send(linchpin_connected);
                }
                StateRequest::GetListenersForModule(topic, sync_module, callback) => {
                    debug!("getting all listeners for module: {:?}", sync_module);
                    let mut listener_list: Vec<SyncSettings> = vec![];
                    if let Some(listener_set) = listener_map.get(topic.as_str()) {
                        for listener in listener_set {
                            if listener.module == sync_module {
                                listener_list.push(listener.to_owned());
                            }
                        }
                        let listener_set: HashSet<SyncSettings> =
                            listener_list.into_iter().collect();
                        listener_list = listener_set.iter().cloned().collect();
                    }
                    debug!("for module listener list: {:?}", listener_list);
                    let _ = callback.send(listener_list);
                }
                StateRequest::SetDistributorToken(token) => sat_token = token.to_owned(),
                StateRequest::GetDistributorToken(callback) => {
                    let _ = callback.send(sat_token.to_owned());
                }
            }
        }
    }
}
///
/// Standard ripple Gateway pattern
/// Start services, and most importantly, start the resolver, which
/// is responsible for looking at input messages and routing them
/// to the correct component for handling
///
///
#[async_trait]
impl Gateway for AppsanityGateway {
    async fn start(mut self: Box<Self>, context: GatewayContext) -> Box<Self> {
        let cloud_services: AppsanityConfig = get_config(&context.config);

        let mut rx = context.receiver;

        let (state_tx, state_rx) = mpsc::channel(32);
        let cloud_sync = CloudPeriodicSync::start(state_tx.clone());
        let cloud_monitor = CloudLinchpinMonitor::start(state_tx.clone());
        let _ = tokio::spawn(async move {
            Self::process_state_requests(state_rx).await;
        });
        let eos_rendered: Arc<TokioMutex<CircularBuffer<Value>>> =
            Arc::new(TokioMutex::new(CircularBuffer::<Value>::new(
                cloud_services.behavioral_metrics.sift.max_queue_size.into(),
            )));
        let session_token = Arc::new(RwLock::new(String::from("")));
        let mut appsanity_resolver = AppsanityServiceResolver::new(
            cloud_services.clone(),
            cloud_sync,
            cloud_monitor,
            eos_rendered.clone(),
            session_token.clone(),
        );

        appsanity_resolver.init(context.session.clone(), context.sender.clone());

        let gateway_receiving_future = async move {
            while let Some(req) = rx.recv().await {
                let _ = appsanity_resolver.resolve(req).await;
            }
        };

        let metrics_send_future = async move {
            /*
            wait send_interval_seconds to start
            */
            debug!("starting BI metrics, send interval is: {}, will wait for send interval before 1st send",cloud_services.behavioral_metrics.sift.send_interval_seconds);
            tokio::time::sleep(time::Duration::from_secs(
                cloud_services
                    .behavioral_metrics
                    .sift
                    .send_interval_seconds
                    .into(),
            ))
            .await;
            loop {
                debug!("sending BI metrics");

                let token = {
                    let token_mutex = session_token.read().await;
                    token_mutex.clone()
                };
                send_metrics(
                    eos_rendered.clone(),
                    cloud_services.behavioral_metrics.sift.endpoint.clone(),
                    token,
                    cloud_services.behavioral_metrics.sift.batch_size.into(),
                )
                .await;
                tokio::time::sleep(time::Duration::from_secs(
                    cloud_services
                        .behavioral_metrics
                        .sift
                        .send_interval_seconds
                        .into(),
                ))
                .await
            }
        };
        let _ = tokio::spawn(metrics_send_future);

        /*
        disembodied future, because tokio
        */
        let _ = tokio::spawn(gateway_receiving_future);

        self
    }

    async fn shutdown(self: Box<Self>) -> bool {
        todo!()
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {

    use dpab_core::message::{DistributorSession, DpabRequest, DpabRequestPayload, DpabResponse};
    use dpab_core::model::auth::AuthRequest;
    use dpab_core::model::auth::GetPlatformTokenParams;
    use tokio::sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    };

    use std::collections::HashMap;

    use super::*;

    #[tokio::test]
    async fn test_gateway_start() {
        let mut passed = false;

        let (dpab_req_tx, dpab_req_rx) = mpsc::channel::<DpabRequest>(32);
        let (dpab_res_tx, dpab_res_rx) = oneshot::channel::<DpabResponse>();

        let get_distributer_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_gateway"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let mut app_ignore_rules = HashMap::new();

        let mut method_ignore_rules = Vec::with_capacity(4);
        method_ignore_rules.push(String::from("GetAdIdObject"));
        method_ignore_rules.push(String::from("GetPlatformToken"));
        method_ignore_rules.push(String::from("GetAppMethodPermission"));

        app_ignore_rules.insert(String::from("App_1"), method_ignore_rules.clone());

        let dpab_tx = dpab_req_tx.clone();
        let th_req = tokio::spawn(async move {
            let get_dpab_request = DpabRequest {
                payload: DpabRequestPayload::Auth(AuthRequest::GetPlatformToken(
                    GetPlatformTokenParams {
                        app_id: String::from("App_1"),
                        dist_session: get_distributer_session.clone(),
                        content_provider: String::from("xumo"),
                        device_session_id: String::from("device_session_id"),
                        app_session_id: String::from("app_session_id"),
                    },
                )),
                callback: Some(dpab_res_tx),
                parent_span: None,
            };

            dpab_tx.send(get_dpab_request).await;
        });
        let config_str = r#""#;

        let gateway_context = GatewayContext {
            session: None,
            sender: dpab_req_tx.clone(),
            receiver: dpab_req_rx,
            config: serde_json::from_str(
                r#" 
            {
                "permission_service": {  
                    "url": "thor-permission.svc-qa.thor.comcast.com"
                },
                "session_service": {
                    "url": "res-api.svc-qa.thor.comcast.com"
                },
                "ad_platform_service": {
                    "url": "ad-platform-service.svc-qa.thor.comcast.com"
                },
                "app_authorization_rules": {
                  "app_ignore_rules": { 
                    "foo-insecure":["*"],
                    "refui": ["*"],
                    "root":["*"],
                    "foo": ["*"]
                  }
                },
                "method_ignore_rules": ["some.nonexistent.method"],
                "behavioral_metrics" : {
                  "sift": {
                    "endpoint": "https://collector.pabs.comcast.com/platco/dev",
                    "api_key": "it's a secret",
                    "batch_size": 1,
                    "max_queue_size": 20,
                    "metrics_schemas": {
                        "default_metrics_namespace" : "entos",
                        "default_metrics_schema_version" : "0",
                        "metrics_schemas": [
                          {"event_name": "example","namespace": "nameit","version": "2.0","alias": "name2useforeventtopabs"}
                        ]
                      }
                  }
                }
          
            }
          "#,
            )
            .unwrap(),
        };

        let th_start = tokio::spawn(async move {
            let mut service = Box::new(AppsanityGateway {});
            service.start(gateway_context).await;
        });

        if let Ok(x) = dpab_res_rx.await {
            passed = true;
        }
        assert!(passed);
    }
    #[tokio::test]
    async fn config_test_happy() {
        let good_config: Value = serde_json::from_str(
            r#" 
        {
            "permission_service": {  
                "url": "thor-permission.svc-qa.thor.comcast.com"
            },
            "session_service": {
                "url": "res-api.svc-qa.thor.comcast.com"
            },
            "ad_platform_service": {
                "url": "ad-platform-service.svc-qa.thor.comcast.com"
            },
            "app_authorization_rules": {
              "app_ignore_rules": { 
                "foo-insecure":["*"],
                "refui": ["*"],
                "root":["*"],
                "foo": ["*"]
              }
            },
            "method_ignore_rules": ["some.nonexistent.method"],
            "behavioral_metrics" : {
              "sift": {
                "endpoint": "https://collector.pabs.comcast.com/platco/dev",
                "api_key": "it's a secret",
                "batch_size": 1,
                "max_queue_size": 20,
                "metrics_schemas": {
                    "default_metrics_namespace" : "entos",
                    "default_metrics_schema_version" : "0",
                    "metrics_schemas": [
                      {"event_name": "example","namespace": "nameit","version": "2.0","alias": "name2useforeventtopabs"}
                    ]
                  }
              }
            }
      
        }
      "#,
        )
        .unwrap();
        let the_config = get_config(&Some(good_config));
    }
    #[tokio::test]
    async fn config_test_none() {
        let the_config = get_config(&None);
    }
    #[tokio::test]
    async fn config_test_sorta_no_aps() {
        let sorta_config: Value = serde_json::from_str(
            r#" 
        {
            "permission_service": {  
                "url": "thor-permission.svc-qa.thor.comcast.com"
            },
            "session_service": {
                "url": "res-api.svc-qa.thor.comcast.com"
            },
            "app_authorization_rules": {
              "app_ignore_rules": { 
                "foo-insecure":["*"],
                "refui": ["*"],
                "root":["*"],
                "foo": ["*"]
              }
            },
            "method_ignore_rules": ["some.nonexistent.method"],
            "behavioral_metrics" : {
              "sift": {
                "endpoint": "https://collector.pabs.comcast.com/platco/dev",
                "api_key": "it's a secret",
                "batch_size": 1,
                "max_queue_size": 20,
                "metrics_schemas": {
                    "default_metrics_namespace" : "entos",
                    "default_metrics_schema_version" : "0",
                    "metrics_schemas": [
                      {"event_name": "example","namespace": "nameit","version": "2.0","alias": "name2useforeventtopabs"}
                    ]
                  }
              }
            }
      
        }
      "#,
        )
        .unwrap();
        let the_config = get_config(&Some(sorta_config));
    }

    async fn config_test_sorta_no_behavioral_config() {
        let sorta_config: Value = serde_json::from_str(
            r#" 
        {
            "permission_service": {  
                "url": "thor-permission.svc-qa.thor.comcast.com"
            },
            "session_service": {
                "url": "res-api.svc-qa.thor.comcast.com"
            },
            "app_authorization_rules": {
              "app_ignore_rules": { 
                "foo-insecure":["*"],
                "refui": ["*"],
                "root":["*"],
                "foo": ["*"]
              }
            },
            "method_ignore_rules": ["some.nonexistent.method"]
            }
      
        }
      "#,
        )
        .unwrap();
        let the_config = get_config(&Some(sorta_config));
    }
}
