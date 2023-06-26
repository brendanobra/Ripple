use std::{
    cmp,
    sync::{Arc, Mutex},
};

use crate::{
    connection_manager::{self},
    model::client_messages::{
        ClientConfig, ClientEvent, ClientRequest, ClientState, ConnectionRequest,
        LinchpinClientError, LinchpinRequest, SetSatRequestData, SubscribeRequestData,
        UnsubscribeRequestData, DEFAULT_HEARTBEAT_TIMEOUT,
    },
};
use tokio::sync::{
    mpsc::{self},
    oneshot,
};
use tracing::{debug, error, info};

const DEFAULT_INITIAL_RECONNECT_DELAY: u64 = 1000;
const DEFAULT_MAX_RECONNECT_DELAY: u64 = 32000;

pub async fn start(
    config: ClientConfig,
    client_event_handler: mpsc::Sender<ClientEvent>,
) -> LinchpinClient {
    let (client_request_tx, client_request_rx) = mpsc::channel::<ClientRequest>(16);
    let (client_event_tx, client_event_rx) = mpsc::channel::<ClientEvent>(16);
    let mut client = LinchpinClient::new(client_request_tx, &config);

    client.init(client_event_rx, client_event_handler);

    tokio::spawn(async move {
        connection_manager::start(config, client_event_tx, client_request_rx).await;
        debug!("linchpin_client: Exiting connection_manager start-up thread");
    });

    client
}

async fn attempt_reconnect(client_request_tx: mpsc::Sender<ClientRequest>, delay_ms: u64) {
    tokio::spawn(async move {
        info!(
            "attempt_reconnect: Sending delayed reconnect request in {} seconds",
            delay_ms / 1000
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        if let Err(e) = client_request_tx
            .send(ClientRequest::Connection(ConnectionRequest::Reconnect))
            .await
        {
            error!("attempt_reconnect: Could not send request: e={:?}", e);
        }
        debug!("attempt_reconnect: Exiting send delayed reconnect thread");
    });
}

pub struct LinchpinClient {
    client_request_tx: mpsc::Sender<ClientRequest>,
    topics: std::vec::Vec<String>,
    state: Arc<Mutex<ClientState>>,
    reconnect_delay_ms: u64,
    max_reconnect_delay_ms: u64,
    heartbeat_timeout_secs: u64,
}

impl LinchpinClient {
    fn new(
        client_request_tx: mpsc::Sender<ClientRequest>,
        config: &ClientConfig,
    ) -> LinchpinClient {
        LinchpinClient {
            client_request_tx,
            topics: std::vec::Vec::new(),
            state: Arc::new(Mutex::new(ClientState::Disconnected(
                LinchpinClientError::Disconnected,
            ))),
            reconnect_delay_ms: config
                .initial_reconnect_delay_ms
                .unwrap_or(DEFAULT_INITIAL_RECONNECT_DELAY),
            max_reconnect_delay_ms: config
                .max_reconnect_delay_ms
                .unwrap_or(DEFAULT_MAX_RECONNECT_DELAY),
            heartbeat_timeout_secs: config
                .heartbeat_timeout_secs
                .unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT),
        }
    }

    fn init(
        &mut self,
        mut client_event_rx: mpsc::Receiver<ClientEvent>,
        client_event_handler: mpsc::Sender<ClientEvent>,
    ) {
        let state = self.state.clone();
        let client_request_tx = self.client_request_tx.clone();
        let max_reconnect_delay_ms = self.max_reconnect_delay_ms;
        let reconnect_delay_ms = Arc::new(Mutex::new(self.reconnect_delay_ms));
        let initial_delay = self.reconnect_delay_ms;

        tokio::spawn(async move {
            let delay_ms = reconnect_delay_ms;
            while let Some(event) = client_event_rx.recv().await {
                debug!("LinchpinClient: Received client event: {:?}", event);

                if let Err(e) = client_event_handler.send(event.clone()).await {
                    error!("LinchpinClient: Could not send event: e={:?}", e);
                }

                if let ClientEvent::State(new_state) = event {
                    *state.lock().unwrap() = new_state.clone();
                    if let ClientState::Disconnected(_) = new_state {
                        // Attempt reconnect with increasing delay.
                        let request_tx = client_request_tx.clone();
                        let delay = *delay_ms.lock().unwrap();
                        attempt_reconnect(request_tx, delay).await;
                        let old_delay = *delay_ms.lock().unwrap();
                        *delay_ms.lock().unwrap() = cmp::min(old_delay * 2, max_reconnect_delay_ms);
                    } else if let ClientState::Connected = new_state {
                        // We've connected, reset the initial delay.
                        *delay_ms.lock().unwrap() = initial_delay;
                    }
                }
            }
            debug!("LinchpinClient: Exiting client_event_rx thread");
        });
    }

    async fn client_request(&self, request: ClientRequest) -> Result<(), LinchpinClientError> {
        if let Err(e) = self.client_request_tx.send(request).await {
            error!("client_request: Could not send request: e={:?}", e);
            return Err(e.into());
        }
        Ok(())
    }

    async fn client_request_await_callback(
        &self,
        request: ClientRequest,
        callback: oneshot::Receiver<Result<(), LinchpinClientError>>,
    ) -> Result<(), LinchpinClientError> {
        self.client_request(request).await?;
        let resp = callback.await;
        match resp {
            Ok(r) => r,
            Err(_) => Err(LinchpinClientError::IoError),
        }
    }

    pub async fn subscribe(&mut self, topic: &String) -> Result<(), LinchpinClientError> {
        if self.topics.contains(&topic) {
            return Err(LinchpinClientError::AlreadySubscribed);
        }

        if *self.state.lock().unwrap() != ClientState::Connected {
            error!("subscribe: Not connected");
            return Err(LinchpinClientError::Disconnected);
        }

        let (callback_tx, callback_rx) = oneshot::channel::<Result<(), LinchpinClientError>>();

        let data = SubscribeRequestData {
            topic: topic.clone(),
            callback: callback_tx,
        };

        match self
            .client_request_await_callback(
                ClientRequest::Linchpin(LinchpinRequest::Subscribe(data)),
                callback_rx,
            )
            .await
        {
            Ok(()) => {
                self.topics.push(topic.clone());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn unsubscribe(&mut self, topic: &String) -> Result<(), LinchpinClientError> {
        if *self.state.lock().unwrap() != ClientState::Connected {
            error!("unsubscribe: Not connected");
            return Err(LinchpinClientError::Disconnected);
        }

        self.topics.retain(|t| !t.eq(topic));
        let (callback_tx, callback_rx) = oneshot::channel::<Result<(), LinchpinClientError>>();

        let data = UnsubscribeRequestData {
            topic: topic.clone(),
            callback: callback_tx,
        };

        if let Err(e) = self
            .client_request_await_callback(
                ClientRequest::Linchpin(LinchpinRequest::Unsubscribe(data)),
                callback_rx,
            )
            .await
        {
            error!("unsubscribe: Could not send request: e={:?}", e);
        }
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), LinchpinClientError> {
        self.client_request(ClientRequest::Connection(ConnectionRequest::Disconnect))
            .await
    }

    pub async fn reconnect(&self) -> Result<(), LinchpinClientError> {
        self.client_request(ClientRequest::Connection(ConnectionRequest::Reconnect))
            .await
    }

    pub async fn set_sat(&self, sat: String) -> Result<(), LinchpinClientError> {
        let (callback_tx, callback_rx) = oneshot::channel::<Result<(), LinchpinClientError>>();
        let data = SetSatRequestData {
            sat,
            callback: callback_tx,
        };
        self.client_request_await_callback(ClientRequest::SetSat(data), callback_rx)
            .await
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use crate::model::client_messages::{
        ClientConfig, ClientEvent, ClientRequest, LinchpinClientError,
    };

    use super::LinchpinClient;

    fn get_default_config() -> ClientConfig {
        ClientConfig {
            url: String::from("someUrl"),
            sat: String::from("someSat"),
            initial_reconnect_delay_ms: None,
            max_reconnect_delay_ms: None,
        }
    }

    fn start_mock(config: ClientConfig) -> LinchpinClient {
        let (client_request_tx, client_request_rx) = mpsc::channel::<ClientRequest>(16);
        let (client_event_tx, client_event_rx) = mpsc::channel::<ClientEvent>(16);
        let client = LinchpinClient::new(client_request_tx, &config);
        client
    }

    #[tokio::test]
    async fn test_unconnected_subscribe() {
        let mut client = start_mock(get_default_config());
        let resp = client.subscribe(&String::from("someTopic")).await;
        assert!(resp.eq(&Err(LinchpinClientError::Disconnected)));
    }

    #[tokio::test]
    async fn test_unconnected_unsubscribe() {
        let mut client = start_mock(get_default_config());
        let resp = client.unsubscribe(&String::from("someTopic")).await;
        assert!(resp.eq(&Err(LinchpinClientError::Disconnected)));
    }

    #[tokio::test]
    async fn test_already_subscribed() {
        let mut client = start_mock(get_default_config());
        let topic = String::from("foo");
        client.topics = vec![topic.clone()];
        let resp = client.subscribe(&topic).await;
        assert!(resp.eq(&Err(LinchpinClientError::AlreadySubscribed)));
    }
}
