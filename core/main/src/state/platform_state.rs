// Copyright 2023 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0
//

use ripple_sdk::{
    api::{
        gateway::rpc_gateway_api::RpcRequest,
        manifest::{
            app_library::AppLibraryState,
            device_manifest::{AppLibraryEntry, DeviceManifest},
            exclusory::ExclusoryImpl,
            extn_manifest::ExtnManifest,
        },
        session::SessionAdjective,
    },
    async_trait::async_trait,
    extn::{
        extn_client_message::{ExtnMessage, ExtnPayloadProvider},
        extn_id::ExtnId,
    },
    framework::ripple_contract::RippleContract,
    utils::error::RippleError,
    uuid::Uuid,
};
use std::{collections::HashMap, sync::Arc};

use crate::{
    broker::{
        endpoint_broker::{BrokerRequest, EndpointBrokerState},
        rules_engine::{RuleEngine, RuleTransformType},
    },
    firebolt::rpc_router::RouterState,
    service::{
        apps::{
            app_events::AppEventsState, delegated_launcher_handler::AppManagerState,
            provider_broker::ProviderBrokerState,
        },
        data_governance::DataGovernanceState,
        extn::ripple_client::RippleClient,
    },
};

use super::{
    cap::cap_state::CapState,
    metrics_state::MetricsState,
    openrpc_state::OpenRpcState,
    ripple_cache::RippleCache,
    session_state::{Session, SessionState},
    types::{ContractualRpcRequest, MetricsProvider, PlatformRpcProvider, PlatformStateProvider},
};
use parking_lot::RwLock;

/// Platform state encapsulates the internal state of the Ripple Main application.
///
/// # Examples
/// ```
/// let state = PlatformState::default();
///
/// let manifest = state.get_device_manifest();
/// println!("{}", manifest.unwrap().configuration.platform);
/// ```
///

#[derive(Debug, Clone)]
pub struct DeviceSessionIdentifier {
    pub device_session_id: Uuid,
}

impl Default for DeviceSessionIdentifier {
    fn default() -> Self {
        Self {
            device_session_id: Uuid::new_v4(),
        }
    }
}
impl From<DeviceSessionIdentifier> for String {
    fn from(device_session_identifier: DeviceSessionIdentifier) -> Self {
        device_session_identifier.device_session_id.to_string()
    }
}
impl From<String> for DeviceSessionIdentifier {
    fn from(uuid_str: String) -> Self {
        DeviceSessionIdentifier {
            device_session_id: Uuid::parse_str(&uuid_str).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformState {
    extn_manifest: ExtnManifest,
    device_manifest: DeviceManifest,
    pub ripple_client: RippleClient,
    pub app_library_state: AppLibraryState,
    pub session_state: SessionState,
    pub cap_state: CapState,
    pub app_events_state: AppEventsState,
    pub provider_broker_state: ProviderBrokerState,
    pub app_manager_state: AppManagerState,
    pub open_rpc_state: OpenRpcState,
    pub router_state: RouterState,
    pub data_governance: DataGovernanceState,
    pub metrics: MetricsState,
    pub device_session_id: DeviceSessionIdentifier,
    pub ripple_cache: RippleCache,
    pub version: Option<String>,
    pub endpoint_state: EndpointBrokerState,
}

impl PlatformState {
    pub fn new(
        extn_manifest: ExtnManifest,
        manifest: DeviceManifest,
        client: RippleClient,
        app_library: Vec<AppLibraryEntry>,
        version: Option<String>,
    ) -> PlatformState {
        let exclusory = ExclusoryImpl::get(&manifest);
        let broker_sender = client.get_broker_sender();
        let rule_engine = RuleEngine::build(&extn_manifest);
        let extn_sdks = extn_manifest.extn_sdks.clone();
        let provider_registations = extn_manifest.provider_registrations.clone();
        let metrics_state = MetricsState::default();
        Self {
            extn_manifest,
            cap_state: CapState::new(manifest.clone()),
            session_state: SessionState::default(),
            device_manifest: manifest.clone(),
            ripple_client: client.clone(),
            app_library_state: AppLibraryState::new(app_library),
            app_events_state: AppEventsState::default(),
            provider_broker_state: ProviderBrokerState::default(),
            app_manager_state: AppManagerState::new(&manifest.configuration.saved_dir),
            open_rpc_state: OpenRpcState::new(Some(exclusory), extn_sdks, provider_registations),
            router_state: RouterState::new(),
            data_governance: DataGovernanceState::default(),
            metrics: metrics_state.clone(),
            device_session_id: DeviceSessionIdentifier::default(),
            ripple_cache: RippleCache::default(),
            version,
            endpoint_state: EndpointBrokerState::new(
                metrics_state,
                broker_sender,
                rule_engine,
                client,
            ),
        }
    }

    pub fn has_internal_launcher(&self) -> bool {
        self.extn_manifest.get_launcher_capability().is_some()
    }

    pub fn get_launcher_capability(&self) -> Option<ExtnId> {
        self.extn_manifest.get_launcher_capability()
    }

    pub fn get_distributor_capability(&self) -> Option<ExtnId> {
        self.extn_manifest.get_distributor_capability()
    }

    pub fn get_manifest(&self) -> ExtnManifest {
        self.extn_manifest.clone()
    }

    pub fn get_rpc_aliases(&self) -> HashMap<String, Vec<String>> {
        self.extn_manifest.clone().rpc_aliases
    }

    pub fn get_device_manifest(&self) -> DeviceManifest {
        self.device_manifest.clone()
    }

    pub fn get_client(&self) -> RippleClient {
        self.ripple_client.clone()
    }

    pub async fn respond(&self, msg: ExtnMessage) -> Result<(), RippleError> {
        self.get_client().respond(msg).await
    }

    pub fn supports_cloud_sync(&self) -> bool {
        let contract = RippleContract::CloudSync.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_encoding(&self) -> bool {
        let contract = RippleContract::Encoder.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_distributor_session(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Distributor).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_session(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Account).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_device_tokens(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Device).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_app_catalog(&self) -> bool {
        let contract = RippleContract::AppCatalog.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_rfc(&self) -> bool {
        let contract = RippleContract::RemoteFeatureControl.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }
    ///
    /// War on dots
    pub async fn internal_rpc_request(
        &self,
        rpc_request: &RpcRequest,
    ) -> Result<ExtnMessage, RippleError> {
        self.get_client()
            .get_extn_client()
            .main_internal_request(rpc_request.to_owned())
            .await
    }
    ///
    /// Also war on dots
    pub async fn extn_request(
        &self,
        rpc_request: impl ExtnPayloadProvider,
    ) -> Result<ExtnMessage, RippleError> {
        self.get_client()
            .send_extn_request(rpc_request.to_owned())
            .await
    }
}
#[derive(Debug, Clone)]
pub struct PlatformStateSingleton {
    extn_manifest: ExtnManifest,
    device_manifest: DeviceManifest,
    pub ripple_client: RippleClient,
    pub app_library_state: AppLibraryState,
    pub session_state: SessionState,
    pub cap_state: CapState,
    pub app_events_state: AppEventsState,
    pub provider_broker_state: ProviderBrokerState,
    pub app_manager_state: AppManagerState,
    pub open_rpc_state: OpenRpcState,
    pub router_state: RouterState,
    pub data_governance: DataGovernanceState,
    pub metrics: MetricsState,
    pub device_session_id: DeviceSessionIdentifier,
    pub ripple_cache: RippleCache,
    pub version: Option<String>,
    pub endpoint_state: EndpointBrokerState,
}
impl PlatformStateSingleton {
    pub fn new(
        extn_manifest: ExtnManifest,
        manifest: DeviceManifest,
        client: RippleClient,
        app_library: Vec<AppLibraryEntry>,
        version: Option<String>,
    ) -> PlatformStateSingleton {
        let exclusory = ExclusoryImpl::get(&manifest);
        let broker_sender = client.get_broker_sender();
        let rule_engine = RuleEngine::build(&extn_manifest);
        let extn_sdks = extn_manifest.extn_sdks.clone();
        let provider_registations = extn_manifest.provider_registrations.clone();
        let metrics_state = MetricsState::default();
        Self {
            extn_manifest,
            cap_state: CapState::new(manifest.clone()),
            session_state: SessionState::default(),
            device_manifest: manifest.clone(),
            ripple_client: client.clone(),
            app_library_state: AppLibraryState::new(app_library),
            app_events_state: AppEventsState::default(),
            provider_broker_state: ProviderBrokerState::default(),
            app_manager_state: AppManagerState::new(&manifest.configuration.saved_dir),
            open_rpc_state: OpenRpcState::new(Some(exclusory), extn_sdks, provider_registations),
            router_state: RouterState::new(),
            data_governance: DataGovernanceState::default(),
            metrics: metrics_state.clone(),
            device_session_id: DeviceSessionIdentifier::default(),
            ripple_cache: RippleCache::default(),
            version,
            endpoint_state: EndpointBrokerState::new(
                metrics_state,
                broker_sender,
                rule_engine,
                client,
            ),
        }
    }

    pub fn has_internal_launcher(&self) -> bool {
        self.extn_manifest.get_launcher_capability().is_some()
    }

    pub fn get_launcher_capability(&self) -> Option<ExtnId> {
        self.extn_manifest.get_launcher_capability()
    }

    pub fn get_distributor_capability(&self) -> Option<ExtnId> {
        self.extn_manifest.get_distributor_capability()
    }

    pub fn get_manifest(&self) -> ExtnManifest {
        self.extn_manifest.clone()
    }

    pub fn get_rpc_aliases(&self) -> HashMap<String, Vec<String>> {
        self.extn_manifest.clone().rpc_aliases
    }

    pub fn get_device_manifest(&self) -> DeviceManifest {
        self.device_manifest.clone()
    }

    pub fn get_client(&self) -> RippleClient {
        self.ripple_client.clone()
    }

    pub async fn respond(&self, msg: ExtnMessage) -> Result<(), RippleError> {
        self.get_client().respond(msg).await
    }

    pub fn supports_cloud_sync(&self) -> bool {
        let contract = RippleContract::CloudSync.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_encoding(&self) -> bool {
        let contract = RippleContract::Encoder.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_distributor_session(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Distributor).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_session(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Account).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_device_tokens(&self) -> bool {
        let contract = RippleContract::Session(SessionAdjective::Device).as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_app_catalog(&self) -> bool {
        let contract = RippleContract::AppCatalog.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }

    pub fn supports_rfc(&self) -> bool {
        let contract = RippleContract::RemoteFeatureControl.as_clear_string();
        self.extn_manifest.required_contracts.contains(&contract)
    }
    ///
    /// War on dots
    pub async fn internal_rpc_request(
        &self,
        rpc_request: &RpcRequest,
    ) -> Result<ExtnMessage, RippleError> {
        self.get_client()
            .get_extn_client()
            .main_internal_request(rpc_request.to_owned())
            .await
    }
    ///
    /// Also war on dots
    pub async fn extn_request(
        &self,
        rpc_request: impl ExtnPayloadProvider,
    ) -> Result<ExtnMessage, RippleError> {
        self.get_client()
            .send_extn_request(rpc_request.to_owned())
            .await
    }
}

impl Default for PlatformStateSingleton {
    fn default() -> Self {
        todo!()
    }
}

#[async_trait]
impl PlatformStateProvider for PlatformStateSingleton {
    fn get_session_for_connection_id(&self, cid: &str) -> Result<Session, RippleError> {
        match self.session_state.get_session_for_connection_id(cid) {
            Some(session) => Ok(session),
            None => Err(RippleError::NoSession),
        }
    }

    fn update_unsubscribe_request(&self, id: u64) -> Result<bool, RippleError> {
        // self.update_unsubscribe_request(id)
        self.endpoint_state.update_unsubscribe_request(id);
        Ok(true)
    }

    // fn get_extn_message(&self, id: u64, is_event: bool) -> Result<ExtnMessage, RippleError> {
    //    // self.get_extn_message(id, is_event)
    //    todo!()
    // }

    fn get_request(&self, id: u64) -> Result<BrokerRequest, RippleError> {
        self.endpoint_state.get_request(id)
    }

    // fn get_transform_data(&self, rule_type: RuleTransformType) -> Option<String> {
    //     //self.get_transform_data(rule_type)
    //     broker_request.rule.transform.get_transform_data
    // }

    fn has_internal_launcher(&self) -> bool {
        self.has_internal_launcher()
    }

    fn get_launcher_capability(&self) -> Option<ExtnId> {
        self.get_launcher_capability()
    }

    fn get_distributor_capability(&self) -> Option<ExtnId> {
        self.get_distributor_capability()
    }
    //add the corresponding self calls for the rest of the functions in this implementation
    fn get_manifest(&self) -> ExtnManifest {
        self.get_manifest()
    }

    fn get_rpc_aliases(&self) -> HashMap<String, Vec<String>> {
        self.get_rpc_aliases()
    }

    fn get_device_manifest(&self) -> DeviceManifest {
        self.get_device_manifest()
    }

    fn get_client(&self) -> RippleClient {
        self.get_client()
    }

    fn supports_cloud_sync(&self) -> bool {
        self.supports_cloud_sync()
    }

    fn supports_encoding(&self) -> bool {
        self.supports_encoding()
    }

    fn supports_distributor_session(&self) -> bool {
        self.supports_distributor_session()
    }

    fn supports_session(&self) -> bool {
        self.supports_session()
    }

    fn supports_device_tokens(&self) -> bool {
        self.supports_device_tokens()
    }

    fn supports_app_catalog(&self) -> bool {
        self.supports_app_catalog()
    }

    fn supports_rfc(&self) -> bool {
        self.supports_rfc()
    }

    fn metrics(&self) -> Arc<RwLock<dyn MetricsProvider>> {
        Arc::new(RwLock::new(self.metrics.clone()))
    }

    fn cache(&self) -> crate::state::ripple_cache::RippleCache {
        self.ripple_cache.clone()
    }

    fn extn_client(&self) -> ripple_sdk::extn::client::extn_client::ExtnClient {
        self.get_client().get_extn_client()
    }

    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    fn internal_rpc_request<'life0, 'life1, 'async_trait>(
        &'life0 self,
        rpc_request: &'life1 RpcRequest,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<ExtnMessage, RippleError>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(self.internal_rpc_request(rpc_request))
    }
    /// Also war on dots
    async fn make_extn_request(
        &self,
        rpc_request: ContractualRpcRequest,
    ) -> Result<ExtnMessage, RippleError> {
        todo!()
    }
}
#[cfg(test)]
mod arc_tests {
    use crate::state::{
        self,
        types::{MockPlatformRpcProvider, MockPlatformStateProvider, PlatformStateProvider},
        *,
    };
    use parking_lot::RwLock;
    use ripple_sdk::tokio;
    use std::sync::Arc;
    #[test]
    pub fn test_basic() {
        use super::*;

        let mut state = MockPlatformStateProvider::default();
        state.expect_supports_encoding().returning(|| true);
        assert_eq!(state.supports_encoding(), true);
        // assert_eq!(state.supports_distributor_session(), true);
        // assert_eq!(state.supports_session(), true);
        // assert_eq!(state.supports_device_tokens(), true);
        // assert_eq!(state.supports_app_catalog(), true);
        // assert_eq!(state.supports_rfc(), true);
    }
    #[test]
    pub fn test_arc() {
        let mut inner = MockPlatformStateProvider::default();
        inner.expect_supports_encoding().returning(|| true);

        let state: Arc<RwLock<dyn PlatformStateProvider>> = Arc::new(RwLock::new(inner));
        let state = state.read();
        assert_eq!(state.supports_encoding(), true);
    }
    #[tokio::test]
    pub async fn test_arc_read_async() {
        let mut inner = MockPlatformStateProvider::default();
        inner.expect_supports_encoding().returning(|| true);

        let state: Arc<RwLock<dyn PlatformStateProvider>> = Arc::new(RwLock::new(inner));
        tokio::spawn(async move {
            let state = state.read();
            assert_eq!(state.supports_encoding(), true);
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    pub async fn test_arc_write_async() {
        let mut inner = MockPlatformStateProvider::default();
        inner.expect_supports_encoding().returning(|| true);

        let state: Arc<RwLock<dyn PlatformStateProvider>> = Arc::new(RwLock::new(inner));
        tokio::spawn(async move {
            let state = state.write();
            assert_eq!(state.supports_encoding(), true);
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    pub async fn test_arc_read_write_async() {
        let mut inner = MockPlatformStateProvider::default();
        inner.expect_supports_encoding().returning(|| true);

        let state: Arc<RwLock<dyn PlatformStateProvider>> = Arc::new(RwLock::new(inner));
        let writer = state.clone();
        tokio::spawn(async move {
            let state = writer.write();
            assert_eq!(state.supports_encoding(), true);
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            let state = state.read();
            assert_eq!(state.supports_encoding(), true);
        })
        .await
        .unwrap();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ripple_sdk::api::manifest::extn_manifest::default_providers;
    use ripple_tdk::utils::test_utils::Mockable;

    impl Mockable for PlatformState {
        fn mock() -> Self {
            use crate::state::bootstrap_state::ChannelsState;

            let (_, manifest) = DeviceManifest::load_from_content(
                include_str!("../../../../examples/manifest/device-manifest-example.json")
                    .to_string(),
            )
            .unwrap();
            let (_, mut extn_manifest) = ExtnManifest::load_from_content(
                include_str!("../../../../examples/manifest/extn-manifest-example.json")
                    .to_string(),
            )
            .unwrap();
            extn_manifest.provider_registrations = default_providers();
            Self::new(
                extn_manifest,
                manifest,
                RippleClient::new(ChannelsState::new()),
                vec![],
                None,
            )
        }
    }
}
