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

use parking_lot::RwLock;
use ripple_sdk::{
    async_trait::async_trait, framework::bootstrap::Bootstep, utils::error::RippleError,
};
use std::sync::Arc;

use crate::broker::endpoint_broker::BrokerOutputForwarder;
use crate::processor::rpc_gateway_processor::RpcGatewayProcessor;
use crate::state::bootstrap_state::BootstrapState;
use crate::state::platform_state::PlatformStateSingleton;
use crate::state::types::PlatformStateProvider;

pub struct StartCommunicationBroker;

#[async_trait]
impl Bootstep<BootstrapState> for StartCommunicationBroker {
    fn get_name(&self) -> String {
        "StartCommunicationBroker".into()
    }

    async fn setup(&self, state: BootstrapState) -> Result<(), RippleError> {
        let ps = state.platform_state.clone();

        let arc: Arc<RwLock<dyn PlatformStateProvider>> =
            Arc::new(RwLock::new(PlatformStateSingleton::new(
                ps.get_manifest(),
                ps.get_device_manifest(),
                ps.get_client(),
                vec![],
                None,
            )));
        // When endpoint broker starts up enable RPC processor there might be internal services which might need
        // brokering data
        state
            .platform_state
            .get_client()
            .add_request_processor(RpcGatewayProcessor::new(state.platform_state.get_client()));

        // Start the Broker Reciever
        if let Ok(rx) = state.channels_state.get_broker_receiver() {
            BrokerOutputForwarder::start_forwarder(ps.clone(), arc.clone(), rx)
        }
        // Setup the endpoints from the manifests
        let mut endpoint_state = ps.clone().endpoint_state;
        endpoint_state.build_thunder_endpoint();
        Ok(())
    }
}

pub struct StartOtherBrokers;

#[async_trait]
impl Bootstep<BootstrapState> for StartOtherBrokers {
    fn get_name(&self) -> String {
        "StartOtherBrokers".into()
    }

    async fn setup(&self, state: BootstrapState) -> Result<(), RippleError> {
        let ps = state.platform_state.clone();
        let arc: Arc<RwLock<dyn PlatformStateProvider>> =
            Arc::new(RwLock::new(PlatformStateSingleton::new(
                ps.get_manifest(),
                ps.get_device_manifest(),
                ps.get_client(),
                vec![],
                None,
            )));
        // Start the Broker Reciever
        if let Ok(rx) = state.channels_state.get_broker_receiver() {
            BrokerOutputForwarder::start_forwarder(ps.clone(), arc.clone(), rx)
        }
        // Setup the endpoints from the manifests
        let mut endpoint_state = ps.clone().endpoint_state;
        endpoint_state.build_other_endpoints(ps.clone(), ps.session_state.get_account_session());
        Ok(())
    }
}
