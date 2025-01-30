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

//pub mod rpc_gateway;
//pub mod firebolt_gateway;
pub mod handlers {
    pub mod accessory_rpc;
    pub mod account_rpc;
    pub mod advertising_rpc;
    pub mod audio_description_rpc;
    pub mod authentication_rpc;
    pub mod capabilities_rpc;
    pub mod closed_captions_rpc;
    pub mod device_rpc;
    pub mod discovery_rpc;
    pub mod keyboard_rpc;
    pub mod lcm_rpc;
    pub mod lifecycle_rpc;
    pub mod localization_rpc;
    pub mod metrics_management_rpc;
    pub mod metrics_rpc;
    pub mod parameters_rpc;
    pub mod privacy_rpc;
    pub mod profile_rpc;
    pub mod provider_registrar;
    pub mod second_screen_rpc;
    pub mod secure_storage_rpc;
    pub mod user_grants_rpc;
    pub mod voice_guidance_rpc;
    pub mod wifi_rpc;
}
pub mod firebolt_gatekeeper;
pub mod firebolt_gateway;
pub mod firebolt_middleware_service;
//pub mod firebolt_ws;
pub mod rpc;
//pub mod rpc_router;

use core::todo;

use ripple_sdk::api::gateway::rpc_gateway_api::RpcRequest;
use ripple_sdk::api::observability::log_signal::LogSignal;
use ripple_sdk::extn::extn_client_message::ExtnMessage;
use ripple_sdk::utils::error::RippleError;
use ripple_sdk::utils::rpc_utils::rpc_custom_error_result;
use ripple_sdk::{utils::rpc_utils::rpc_custom_error, JsonRpcErrorType};

use crate::state::platform_state::PlatformState;
use crate::utils::router_utils::return_extn_response;
fn no_value_returned_error<T>(module: &str, method: &str) -> Result<T, JsonRpcErrorType> {
    rpc_custom_error_result(format!("device.{} error: no value returned", method))
}
fn invalid_device_response_error<T>(
    module: &str,
    method: &str,
    error: RippleError,
) -> Result<T, JsonRpcErrorType> {
    rpc_custom_error_result(format!("device.{} error: {}", method, error))
}
pub async fn route_extn_protocol(state: &PlatformState, req: RpcRequest, extn_msg: ExtnMessage) {
    //let methods = state.router_state.get_methods();
    //let resources = state.router_state.resources.clone();

    let mut platform_state = state.clone();
    LogSignal::new(
        "rpc_router".to_string(),
        "route_extn_protocol".into(),
        req.clone(),
    )
    .emit_debug();
    todo!("route_extn_protocol");
    // ripple_sdk::tokio::spawn(async move {
    //     if let Ok(msg) = resolve_route(&mut platform_state, methods, resources, req).await {
    //         return_extn_response(msg, extn_msg);
    //     }
    // });
}
