use mockall::{automock, mock};
use parking_lot::RwLock;
use ripple_sdk::{async_trait::async_trait, extn::extn_client_message::ExtnPayload};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    broker::{
        endpoint_broker::BrokerRequest,
        rules_engine::{RuleEngine, RuleTransformType},
    },
    firebolt::{handlers::audio_description_rpc, rpc_router::RouterState},
    service::{
        apps::{
            app_events::AppEventsState, delegated_launcher_handler::AppManagerState,
            provider_broker::ProviderBrokerState,
        },
        data_governance::DataGovernanceState,
        extn::ripple_client::RippleClient,
    },
};
use ripple_sdk::{
    api::{
        gateway::rpc_gateway_api::RpcRequest,
        manifest::{
            app_library::AppLibraryState,
            device_manifest::{AppLibraryEntry, DeviceManifest},
            exclusory::ExclusoryImpl,
            extn_manifest::ExtnManifest,
        },
        observability::metrics_util::ApiStats,
        session::SessionAdjective,
    },
    extn::{
        extn_client_message::{ExtnMessage, ExtnPayloadProvider},
        extn_id::ExtnId,
    },
    framework::ripple_contract::RippleContract,
    utils::error::RippleError,
    uuid::Uuid,
};

use super::session_state::Session;

#[automock]
pub trait MetricsProvider: Send + Sync {
    fn remove_api_stats(&self, request_id: &str);
    fn get_api_stats(&self, request_id: &str) -> Option<ApiStats>;
    fn update_api_stats_ref(&self, request_id: &str, stats_ref: Option<String>);
    fn update_api_stage(&self, request_id: &str, stage: &str) -> i64;
    fn add_api_stats(&self, request_id: &str, api: &str) -> ();
}

#[automock]
#[async_trait]
pub trait PlatformStateProvider: Send + Sync {
    /*
    acessors
    */
    fn get_session_for_connection_id(&self, cid: &str) -> Result<Session, RippleError>;
    fn update_unsubscribe_request(&self, id: u64) -> Result<bool, RippleError>;
    fn get_extn_message(&self, id: u64, is_event: bool) -> Result<ExtnMessage, RippleError>;
    fn get_request(&self, id: u64) -> Result<BrokerRequest, RippleError>;
    fn get_transform_data(&self, rule_type: RuleTransformType) -> Option<String>;

    fn has_internal_launcher(&self) -> bool;

    fn get_launcher_capability(&self) -> Option<ExtnId>;

    fn get_distributor_capability(&self) -> Option<ExtnId>;

    fn get_manifest(&self) -> ExtnManifest;

    fn get_rpc_aliases(&self) -> HashMap<String, Vec<String>>;

    fn get_device_manifest(&self) -> DeviceManifest;

    fn get_client(&self) -> RippleClient;
    //async fn respond(&self, msg: ExtnMessage) -> Result<(), RippleError>;

    fn supports_cloud_sync(&self) -> bool;

    fn supports_encoding(&self) -> bool;

    fn supports_distributor_session(&self) -> bool;

    fn supports_session(&self) -> bool;

    fn supports_device_tokens(&self) -> bool;

    fn supports_app_catalog(&self) -> bool;

    fn supports_rfc(&self) -> bool;

    fn metrics(&self) -> Arc<RwLock<dyn MetricsProvider>>;
    fn cache(&self) -> crate::state::ripple_cache::RippleCache;
    fn extn_client(&self) -> ripple_sdk::extn::client::extn_client::ExtnClient;

    // fn remove_api_stats(&self, request_id: &str);
    // fn get_api_stats(&self, request_id: &str) -> Option<ApiStats>;
    // fn update_api_stats_ref(&self, request_id: &str, stats_ref: Option<String>);
    // fn update_api_stage(&self, request_id: &str, stage: &str) -> i64;
    // fn add_api_stats(&self, request_id: &str, api: &str) -> ();

    async fn internal_rpc_request(
        &self,
        rpc_request: &RpcRequest,
    ) -> Result<ExtnMessage, RippleError>;
    ///
    /// Also war on dots
    async fn make_extn_request(
        &self,
        rpc_request: ContractualRpcRequest,
    ) -> Result<ExtnMessage, RippleError>;
}
// pub trait NotPlatformStateProvider: PlatformStateHolder + Send + Sync {

// }
#[automock]
#[async_trait]
pub trait PlatformRpcProvider {
    ///
    /// War on dots
    async fn internal_rpc_request(
        &self,
        rpc_request: &RpcRequest,
    ) -> Result<ExtnMessage, RippleError>;
    ///
    /// Also war on dots
    async fn make_extn_request(
        &self,
        rpc_request: ContractualRpcRequest,
    ) -> Result<ExtnMessage, RippleError>;
}
#[derive(Clone, Debug)]
pub struct ContractualRpcRequest {
    pub contract: RippleContract,
    pub request: ExtnPayload,
}

pub trait RipplePlatform: PlatformStateProvider + MetricsProvider + PlatformRpcProvider {}
// mock! {
//     pub ingRipplePlatform{}
//     #[async_trait]
//     impl PlatformStateProvider for ingRipplePlatform{
//         fn get_session_for_connection_id(&self, cid: &str) -> Result<Session, RippleError>;
//         fn update_unsubscribe_request(&self, id: u64) -> Result<bool, RippleError>;
//         fn get_extn_message(&self, id: u64, is_event: bool) -> Result<ExtnMessage, RippleError>;
//         fn get_request(&self, id: u64) -> Result<BrokerRequest, RippleError>;
//         fn get_transform_data(&self, rule_type: RuleTransformType) -> Option<String>;

//         fn has_internal_launcher(&self) -> bool;

//         fn get_launcher_capability(&self) -> Option<ExtnId>;

//         fn get_distributor_capability(&self) -> Option<ExtnId>;

//         fn get_manifest(&self) -> ExtnManifest;

//         fn get_rpc_aliases(&self) -> HashMap<String, Vec<String>>;

//         fn get_device_manifest(&self) -> DeviceManifest;

//         fn get_client(&self) -> RippleClient;
//          //async fn respond(&self, msg: ExtnMessage) -> Result<(), RippleError>;

//         fn supports_cloud_sync(&self) -> bool;

//         fn supports_encoding(&self) -> bool;

//         fn supports_distributor_session(&self) -> bool;

//         fn supports_session(&self) -> bool;

//         fn supports_device_tokens(&self) -> bool;

//         fn supports_app_catalog(&self) -> bool;

//         fn supports_rfc(&self) -> bool;

//     }
//     #[async_trait]
//     impl MetricsProvider for ingRipplePlatform{
//         fn remove_api_stats(&mut self, request_id: &str);
//         fn get_api_stats(&self, request_id: &str) -> Option<ApiStats>;
//         fn update_api_stats_ref(&mut self, request_id: &str, stats_ref: Option<String>);
//         fn update_api_stage(&mut self, request_id: &str, stage: &str) -> i64;
//         fn add_api_stats(&mut self, request_id: &str, api: &str) -> ();
//     }
//     #[async_trait]
//     impl PlatformRpcProvider for ingRipplePlatform{
//         async fn internal_rpc_request(
//             &self,
//             rpc_request: &RpcRequest,
//         ) -> Result<ExtnMessage, RippleError>;
//         async fn make_extn_request(
//             &self,
//             rpc_request: ContractualRpcRequest,
//         ) -> Result<ExtnMessage, RippleError>;
//     }
//     impl RipplePlatform for ingRipplePlatform{}
// }
