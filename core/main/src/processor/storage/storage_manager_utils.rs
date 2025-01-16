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
    api::device::device_peristence::StorageData,
    extn::extn_client_message::ExtnResponse,
    serde_json::{self, Value},
    utils::{
        error::RippleError,
        rpc_utils::{rpc_custom_error, rpc_custom_error_result},
    },
    JsonRpcErrorType,
};

fn storage_error<T>() -> Result<T, JsonRpcErrorType> {
    rpc_custom_error_result("error parsing response")
}

fn get_storage_data(
    resp: Result<ExtnResponse, RippleError>,
) -> Result<Option<StorageData>, JsonRpcErrorType> {
    match resp {
        Ok(response) => match response {
            ExtnResponse::StorageData(storage_data) => Ok(Some(storage_data)),
            _ => Ok(None),
        },
        Err(_) => storage_error(),
    }
}

fn get_value(resp: Result<ExtnResponse, RippleError>) -> Result<Value, JsonRpcErrorType> {
    let has_storage_data = get_storage_data(resp.clone())?;

    if let Some(storage_data) = has_storage_data {
        return Ok(storage_data.value);
    }

    match resp.unwrap() {
        ExtnResponse::Value(value) => Ok(value),
        ExtnResponse::String(str_val) => match serde_json::from_str(&str_val) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Value::String(str_val)), // An actual string was stored, return it as a Value.
        },
        _ => storage_error(),
    }
}

pub fn storage_to_string_rpc_result(resp: Result<ExtnResponse, RippleError>) -> RpcResult<String> {
    let value = get_value(resp)?;

    if let Some(s) = value.as_str() {
        return Ok(s.to_string());
    }

    storage_error()
}

pub fn storage_to_bool_rpc_result(resp: Result<ExtnResponse, RippleError>) -> RpcResult<bool> {
    let value = get_value(resp)?;

    if let Some(b) = value.as_bool() {
        return Ok(b);
    }
    if let Some(s) = value.as_str() {
        return Ok(s == "true");
    }

    storage_error()
}

pub fn storage_to_u32_rpc_result(resp: Result<ExtnResponse, RippleError>) -> RpcResult<u32> {
    let value = get_value(resp)?;
    if let Some(n) = value.as_u64() {
        return Ok(n as u32);
    }
    if let Some(s) = value.as_str() {
        return s.parse::<u32>().map_or(storage_error(), Ok);
    }

    storage_error()
}

pub fn storage_to_f32_rpc_result(resp: Result<ExtnResponse, RippleError>) -> RpcResult<f32> {
    let value = get_value(resp)?;

    if let Some(n) = value.as_f64() {
        return Ok(n as f32);
    }
    if let Some(s) = value.as_str() {
        return s.parse::<f64>().map_or(storage_error(), |v| Ok(v as f32));
    }

    storage_error()
}

pub fn storage_to_void_rpc_result(resp: Result<ExtnResponse, RippleError>) -> RpcResult<()> {
    match resp {
        Ok(_) => Ok(()),
        Err(_) => storage_error(),
    }
}

pub fn storage_to_vec_string_rpc_result(
    resp: Result<ExtnResponse, RippleError>,
) -> RpcResult<Vec<String>> {
    let value = get_value(resp)?;
    match serde_json::from_value(value) {
        Ok(v) => Ok(v),
        Err(_) => storage_error(),
    }
}
