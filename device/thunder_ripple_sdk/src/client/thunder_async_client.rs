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

use super::{
    device_operator::{DeviceChannelRequest, DeviceResponseMessage},
    thunder_async_client_plugins_status_mgr::{AsyncCallback, AsyncSender, StatusManager},
};
use crate::utils::get_next_id;
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ripple_sdk::{
    api::gateway::rpc_gateway_api::{JsonRpcApiRequest, JsonRpcApiResponse},
    log::{debug, error, info},
    tokio::{self, net::TcpStream, sync::mpsc::Receiver},
    utils::{error::RippleError, rpc_utils::extract_tcp_port},
};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use tokio_tungstenite::{client_async, tungstenite::Message, WebSocketStream};

#[derive(Clone, Debug)]
pub struct ThunderAsyncClient {
    status_manager: StatusManager,
    sender: AsyncSender,
    callback: AsyncCallback,
    subscriptions: HashMap<String, JsonRpcApiRequest>,
}

#[derive(Clone, Debug)]
pub struct ThunderAsyncRequest {
    pub id: u64,
    request: DeviceChannelRequest,
}

impl ThunderAsyncRequest {
    pub fn new(request: DeviceChannelRequest) -> Self {
        Self {
            id: get_next_id(),
            request,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ThunderAsyncResponse {
    pub id: Option<u64>,
    pub result: Result<JsonRpcApiResponse, RippleError>,
}

impl ThunderAsyncClient {}

impl ThunderAsyncResponse {
    fn new_response(response: JsonRpcApiResponse) -> Self {
        Self {
            id: response.id,
            result: Ok(response),
        }
    }

    fn new_error(id: u64, e: RippleError) -> Self {
        Self {
            id: Some(id),
            result: Err(e),
        }
    }

    pub fn get_method(&self) -> Option<String> {
        if let Ok(e) = &self.result {
            return e.method.clone();
        }
        None
    }

    pub fn get_id(&self) -> Option<u64> {
        match &self.result {
            Ok(response) => response.id,
            Err(_) => None,
        }
    }

    pub fn get_device_resp_msg(&self, sub_id: Option<String>) -> Option<DeviceResponseMessage> {
        let json_resp = match &self.result {
            Ok(json_resp_res) => json_resp_res,
            _ => return None,
        };
        DeviceResponseMessage::create(json_resp, sub_id)
    }
}

impl ThunderAsyncClient {
    pub fn get_sender(&self) -> AsyncSender {
        self.sender.clone()
    }

    pub fn get_callback(&self) -> AsyncCallback {
        self.callback.clone()
    }
    async fn create_ws(
        endpoint: &str,
    ) -> (
        SplitSink<WebSocketStream<TcpStream>, Message>,
        SplitStream<WebSocketStream<TcpStream>>,
    ) {
        debug!("create_ws: {}", endpoint);
        let port = extract_tcp_port(endpoint);
        let tcp_port = port.unwrap();
        let mut index = 0;

        loop {
            // Try connecting to the tcp port first
            if let Ok(v) = TcpStream::connect(&tcp_port).await {
                debug!("create_ws: Connected");
                // Setup handshake for websocket with the tcp port
                // Some WS servers lock on to the Port but not setup handshake till they are fully setup
                if let Ok((stream, _)) = client_async(endpoint, v).await {
                    break stream.split();
                }
            }
            if (index % 10).eq(&0) {
                error!(
                    "create_ws: endpoint {} failed with retry for last {} secs in {}",
                    endpoint, index, tcp_port
                );
            }
            index += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    fn prepare_request(&self, request: &ThunderAsyncRequest) -> Result<String, RippleError> {
        let id: u64 = request.id;
        let (callsign, method) = request.request.get_callsign_method();

        // Check if the method is empty and return an error if it is
        if method.is_empty() {
            return Err(RippleError::InvalidInput);
        }

        // Check the status of the plugin using the status manager
        let status = match self.status_manager.get_status(callsign.clone()) {
            Some(v) => v.clone(),
            None => {
                // If the plugin status is not available, add the request to the pending list
                self.status_manager
                    .add_async_client_request_to_pending_list(callsign.clone(), request.clone());
                // Generate a request to check the plugin status and add it to the requests list
                let request = self
                    .status_manager
                    .generate_plugin_status_request(callsign.clone());
                return Ok(request.to_string());
            }
        };

        // If the plugin is missing, return a service error
        if status.state.is_missing() {
            error!("Plugin {} is missing", callsign);
            return Err(RippleError::ServiceError);
        }

        // If the plugin is activating, return a service not ready error
        if status.state.is_activating() {
            info!("Plugin {} is activating", callsign);
            return Err(RippleError::ServiceNotReady);
        }

        // If the plugin is not activated, add the request to the pending list and generate an activation request
        if !status.state.is_activated() {
            self.status_manager
                .add_async_client_request_to_pending_list(callsign.clone(), request.clone());
            let request = self
                .status_manager
                .generate_plugin_activation_request(callsign.clone());
            return Ok(request.to_string());
        }

        // Generate the appropriate JSON-RPC request based on the type of DeviceChannelRequest
        let r = match &request.request {
            DeviceChannelRequest::Call(c) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": c.method,
                "params": c.params
            })
            .to_string(),
            DeviceChannelRequest::Unsubscribe(_) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": format!("{}.unregister", callsign),
                "params": {
                    "event": method,
                    "id": "client.events"
                }
            })
            .to_string(),
            DeviceChannelRequest::Subscribe(_) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": format!("{}.register", callsign),
                "params": json!({
                    "event": method,
                    "id": "client.events"
                })
            })
            .to_string(),
        };

        Ok(r)
    }

    pub fn new(callback: AsyncCallback, sender: AsyncSender) -> Self {
        Self {
            status_manager: StatusManager::new(),
            sender,
            callback,
            subscriptions: HashMap::new(),
        }
    }

    pub async fn process_new_req(&mut self, request: String, url: String) {
        let (mut ws_tx, mut ws_rx) = Self::create_ws(&url).await;
        let _feed = ws_tx
            .feed(tokio_tungstenite::tungstenite::Message::Text(request))
            .await;
        let _flush = ws_tx.flush().await;

        if let Some(resp) = ws_rx.next().await {
            match resp {
                Ok(message) => {
                    self.handle_response(message).await;
                    //close the newly created websocket
                    let _ = ws_tx.close().await;
                }
                Err(e) => {
                    error!("thunder_async_client Websocket error on read {:?}", e);
                }
            }
        }
    }

    async fn handle_response(&mut self, message: Message) {
        if let Message::Text(t) = message {
            let request = t.as_bytes();
            //check controller response or not
            if self
                .status_manager
                .is_controller_response(self.get_sender(), self.callback.clone(), request)
                .await
            {
                self.status_manager
                    .handle_controller_response(self.get_sender(), self.callback.clone(), request)
                    .await;
            } else {
                self.handle_jsonrpc_response(request).await
            }
        }
    }

    async fn process_subscribe_requests(
        &mut self,
        ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) {
        for (_, subscription_request) in self.subscriptions.iter_mut() {
            let new_id = get_next_id();

            debug!(
                "process_subscribe_requests: method={}, params={:?}, old_id={:?}, new_id={}",
                subscription_request.method,
                subscription_request.params,
                subscription_request.id,
                new_id
            );

            subscription_request.id = Some(new_id);

            let request_json = serde_json::to_string(&subscription_request).unwrap();
            let _feed = ws_tx
                .feed(tokio_tungstenite::tungstenite::Message::Text(request_json))
                .await;
        }
    }

    pub async fn start(
        &mut self,
        url: &str,
        mut thunder_async_request_rx: Receiver<ThunderAsyncRequest>,
    ) {
        loop {
            info!("start: (re)establishing websocket connection: url={}", url);

            let (mut subscriptions_tx, mut subscriptions_rx) = Self::create_ws(url).await;

            // send the controller statechange subscription request
            let status_request = self
                .status_manager
                .generate_state_change_subscribe_request();

            let _feed = subscriptions_tx
                .feed(tokio_tungstenite::tungstenite::Message::Text(
                    status_request.to_string(),
                ))
                .await;

            self.process_subscribe_requests(&mut subscriptions_tx).await;

            let _flush = subscriptions_tx.flush().await;

            tokio::pin! {
                let subscriptions_socket = subscriptions_rx.next();
            }

            loop {
                tokio::select! {
                    Some(value) = &mut subscriptions_socket => {
                        match value {
                            Ok(message) => {
                                self.handle_response(message).await;
                            },
                            Err(e) => {
                                error!("Thunder_async_client Websocket error on read {:?}", e);
                                break;
                            }
                        }
                    },
                    Some(request) = thunder_async_request_rx.recv() => {
                        debug!("thunder_async_request_rx: request={:?}", request);
                        // here prepare_request will check the plugin status and add json rpc format
                        match self.prepare_request(&request) {
                            Ok(updated_request) => {
                                if let Ok(jsonrpc_request) = serde_json::from_str::<JsonRpcApiRequest>(&updated_request) {
                                    if jsonrpc_request.method.ends_with(".register") {
                                        if let Some(Value::Object(ref params)) = jsonrpc_request.params {
                                            if let Some(Value::String(event)) = params.get("event") {
                                                debug!("thunder_async_request_rx: Rerouting subscription request for {}", event);

                                                // Store the subscription request in the subscriptions list in case we need to
                                                // resubscribe later due to a socket disconnect.
                                                self.subscriptions.insert(event.to_string(), jsonrpc_request.clone());

                                                // Reroute subsubscription requests through the persistent websocket so all notifications
                                                // are sent to the same websocket connection.
                                                let _feed = subscriptions_tx.feed(tokio_tungstenite::tungstenite::Message::Text(updated_request)).await;
                                                let _flush = subscriptions_tx.flush().await;
                                            } else {
                                                error!("thunder_async_request_rx: Missing 'event' parameter");
                                            }
                                        } else {
                                            error!("thunder_async_request_rx: Missing 'params' object");
                                        }
                                    } else {
                                        // TODO: I don't like that we have to clone the client, we should refactor this. -pca
                                        let mut thunder_async_client = self.clone();
                                            let url_clone = url.to_string();
                                            tokio::spawn(async move {
                                                thunder_async_client.process_new_req(updated_request, url_clone).await;
                                                }
                                            );
                                    }
                                }
                            }
                            Err(e) => {
                                let response = ThunderAsyncResponse::new_error(request.id,e.clone());
                                match e {
                                    RippleError::ServiceNotReady => {
                                        info!("Thunder Service not ready, request is now in pending list {:?}", request);
                                    },
                                    _ => {
                                        error!("error preparing request {:?}", e)
                                    }
                                }
                                self.callback.send(response).await;
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_jsonrpc_response(&mut self, result: &[u8]) {
        if let Ok(message) = serde_json::from_slice::<JsonRpcApiResponse>(result) {
            self.callback
                .send(ThunderAsyncResponse::new_response(message))
                .await
        } else {
            error!("handle_jsonrpc_response: Invalid JSON RPC message sent by Thunder");
        }
    }

    pub async fn send(&self, request: ThunderAsyncRequest) {
        if let Err(e) = self.sender.send(request).await {
            error!("Failed to send thunder Async Request: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::device_operator::DeviceCallRequest;
    use ripple_sdk::api::gateway::rpc_gateway_api::JsonRpcApiResponse;
    use ripple_sdk::utils::error::RippleError;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_thunder_async_request_new() {
        let callrequest = DeviceCallRequest {
            method: "org.rdk.System.1.getSerialNumber".to_string(),
            params: None,
        };

        let request = DeviceChannelRequest::Call(callrequest);
        let _async_request = ThunderAsyncRequest::new(request.clone());
        assert_eq!(
            _async_request.request.get_callsign_method(),
            request.get_callsign_method()
        );
    }

    #[tokio::test]
    async fn test_thunder_async_response_new_response() {
        let response = JsonRpcApiResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(6),
            result: Some(json!({"key": "value"})),
            error: None,
            method: None,
            params: None,
        };

        let _async_response = ThunderAsyncResponse::new_response(response.clone());
        assert_eq!(_async_response.result.unwrap().result, response.result);
    }

    #[tokio::test]
    async fn test_thunder_async_response_new_error() {
        let error = RippleError::ServiceError;
        let async_response = ThunderAsyncResponse::new_error(1, error.clone());
        assert_eq!(async_response.id, Some(1));
        assert_eq!(async_response.result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_thunder_async_response_get_event() {
        let response = JsonRpcApiResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(6),
            result: Some(json!({"key": "value"})),
            error: None,
            method: Some("event_1".to_string()),
            params: None,
        };
        let async_response = ThunderAsyncResponse::new_response(response);
        assert_eq!(async_response.get_method(), Some("event_1".to_string()));
    }

    #[tokio::test]
    async fn test_thunder_async_response_get_id() {
        let response = JsonRpcApiResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(42),
            result: Some(json!({"key": "value"})),
            error: None,
            method: Some("event_1".to_string()),
            params: None,
        };
        let async_response = ThunderAsyncResponse::new_response(response);
        assert_eq!(async_response.get_id(), Some(42));
    }

    #[tokio::test]
    async fn test_thunder_async_response_get_device_resp_msg() {
        let response = JsonRpcApiResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(6),
            result: Some(json!({"key": "value"})),
            error: None,
            method: Some("event_1".to_string()),
            params: None,
        };
        let async_response = ThunderAsyncResponse::new_response(response);
        let device_resp_msg = async_response.get_device_resp_msg(None);
        assert_eq!(device_resp_msg.unwrap().message, json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_thunder_async_client_prepare_request() {
        let (resp_tx, _resp_rx) = mpsc::channel(10);
        let callback = AsyncCallback { sender: resp_tx };
        let (async_tx, _async_rx) = mpsc::channel(10);
        let async_sender = AsyncSender { sender: async_tx };
        let client = ThunderAsyncClient::new(callback, async_sender);

        let callrequest = DeviceCallRequest {
            method: "org.rdk.System.1.getSerialNumber".to_string(),
            params: None,
        };

        let request = DeviceChannelRequest::Call(callrequest);
        let async_request = ThunderAsyncRequest::new(request);
        let result = client.prepare_request(&async_request);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_thunder_async_client_send() {
        let (resp_tx, _resp_rx) = mpsc::channel(10);
        let callback = AsyncCallback { sender: resp_tx };
        let (async_tx, mut async_rx) = mpsc::channel(10);
        let async_sender = AsyncSender { sender: async_tx };
        let client = ThunderAsyncClient::new(callback, async_sender);

        let callrequest = DeviceCallRequest {
            method: "org.rdk.System.1.getSerialNumber".to_string(),
            params: None,
        };

        let request = DeviceChannelRequest::Call(callrequest);
        let async_request = ThunderAsyncRequest::new(request);
        client.send(async_request.clone()).await;
        let received = async_rx.recv().await;
        assert_eq!(received.unwrap().id, async_request.id);
    }

    #[tokio::test]
    async fn test_thunder_async_client_handle_jsonrpc_response() {
        let (resp_tx, mut resp_rx) = mpsc::channel(10);
        let callback = AsyncCallback { sender: resp_tx };
        let response = JsonRpcApiResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(6),
            result: Some(json!({"key": "value"})),
            error: None,
            method: Some("event_1".to_string()),
            params: None,
        };
        let response_bytes = serde_json::to_vec(&response).unwrap();
        let (async_tx, _async_rx) = mpsc::channel(1);
        let async_sender = AsyncSender { sender: async_tx };
        let mut client = ThunderAsyncClient::new(callback, async_sender);
        client.handle_jsonrpc_response(&response_bytes).await;

        let received = resp_rx.recv().await;
        assert_eq!(
            received.unwrap().result.unwrap().result,
            Some(json!({"key": "value"}))
        );
    }

    #[tokio::test]
    async fn test_thunder_async_client_start() {
        let (resp_tx, mut resp_rx) = mpsc::channel(10);
        let callback = AsyncCallback { sender: resp_tx };
        let (async_tx, _async_rx) = mpsc::channel(10);
        let async_sender = AsyncSender { sender: async_tx };
        let mut client = ThunderAsyncClient::new(callback.clone(), async_sender);

        let response = json!({
            "jsonrpc": "2.0",
            "result": {
                "key": "value"
            }
        });

        client
            .handle_jsonrpc_response(response.to_string().as_bytes())
            .await;
        let received = resp_rx.recv().await;
        assert!(received.is_some());
        let async_response = received.unwrap();
        assert_eq!(
            async_response.result.unwrap().result,
            Some(json!({"key": "value"}))
        );
    }
}
