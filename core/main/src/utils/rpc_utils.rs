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

use jsonrpsee::core::RpcResult;

use ripple_sdk::{
    api::{
        firebolt::fb_general::{ListenRequest, ListenerResponse},
        gateway::rpc_gateway_api::CallContext,
    },
    tokio::sync::oneshot,
    utils::rpc_utils::{rpc_custom_error, rpc_custom_error_result, rpc_error_with_code_result},
    JsonRpcErrorType,
};

use crate::{
    firebolt::firebolt_gateway::JsonRpcError,
    service::apps::app_events::{AppEventDecorator, AppEvents},
    state::platform_state::PlatformState,
};

pub use ripple_sdk::utils::rpc_utils::rpc_err;

pub const FIRE_BOLT_DEEPLINK_ERROR_CODE: i32 = -40400;
pub const DOWNSTREAM_SERVICE_UNAVAILABLE_ERROR_CODE: i32 = -50200;
pub const SESSION_NO_INTENT_ERROR_CODE: i32 = -40000;

/// Awaits a oneshot to respond. If the oneshot fails to repond, creates a generic
/// RPC internal error
pub async fn rpc_await_oneshot<T>(rx: oneshot::Receiver<T>) -> RpcResult<T> {
    match rx.await {
        Ok(v) => Ok(v),
        Err(e) => rpc_custom_error_result(format!("Internal failure: {:?}", e)),
    }
}

/// listener for events any events.
pub async fn rpc_add_event_listener(
    state: &PlatformState,
    ctx: CallContext,
    request: ListenRequest,
    event_name: &'static str,
) -> RpcResult<ListenerResponse> {
    let listen = request.listen;

    AppEvents::add_listener(state, event_name.to_string(), ctx, request);
    Ok(ListenerResponse {
        listening: listen,
        event: event_name.into(),
    })
}

/// listener for events any events.
pub async fn rpc_add_event_listener_with_decorator(
    state: &PlatformState,
    ctx: CallContext,
    request: ListenRequest,
    event_name: &'static str,
    decorator: Option<Box<dyn AppEventDecorator + Send + Sync>>,
) -> RpcResult<ListenerResponse> {
    let listen = request.listen;

    AppEvents::add_listener_with_decorator(state, event_name.to_string(), ctx, request, decorator);
    Ok(ListenerResponse {
        listening: listen,
        event: event_name.into(),
    })
}

pub fn rpc_downstream_service_err<T>(msg: &str) -> Result<T, JsonRpcErrorType> {
    rpc_error_with_code_result(msg.to_string(), DOWNSTREAM_SERVICE_UNAVAILABLE_ERROR_CODE)
}
pub fn rpc_session_no_intent_err<T>(msg: &str) -> Result<T, JsonRpcErrorType> {
    rpc_error_with_code_result(msg.to_string(), SESSION_NO_INTENT_ERROR_CODE)
}
pub fn rpc_navigate_reserved_app_err<T>(msg: &str) -> Result<T, JsonRpcErrorType> {
    rpc_error_with_code_result(msg.to_string(), FIRE_BOLT_DEEPLINK_ERROR_CODE)
}

pub fn get_base_method(method: &str) -> String {
    let method_vec: Vec<&str> = method.split('.').collect();
    method_vec.first().unwrap().to_string().to_lowercase()
}

pub fn extract_tcp_port(url: &str) -> String {
    let url_split: Vec<&str> = url.split("://").collect();
    if let Some(domain) = url_split.get(1) {
        let domain_split: Vec<&str> = domain.split('/').collect();
        domain_split.first().unwrap().to_string()
    } else {
        url.to_owned()
    }
}
