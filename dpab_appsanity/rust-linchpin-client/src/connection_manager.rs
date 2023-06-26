use std::collections::HashMap;

use std::time::{Duration, Instant};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    linchpin_connection::{self},
    model::client_messages::{
        ClientConfig, ClientEvent, ClientRequest, ClientState, ConnectionRequest,
        LinchpinClientError, LinchpinRequest, SetSatRequestData, SubscribeRequestData,
        UnsubscribeRequestData, DEFAULT_HEARTBEAT_TIMEOUT,
    },
    model::linchpin_messages::{
        ListenAckMessage, Message, MessageType, NotificationType, Operation, STATUS_CODE_OK,
    },
};

pub async fn open_socket(
    url: &String,
    sat: &String,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, LinchpinClientError> {
    let sat_header = format!("SAT_WEB_SOCKET_EXTENSION; sat={}", sat);
    let mut u = url.into_client_request()?;
    u.headers_mut().insert(
        "Sec-WebSocket-Extensions",
        HeaderValue::from_str(sat_header.as_str()).unwrap(),
    );

    let (socket, _) = connect_async(u).await?;
    Ok(socket)
}

pub struct ConnectionManager {
    config: ClientConfig,
    client_event_tx: mpsc::Sender<ClientEvent>,
    client_request_rx: mpsc::Receiver<ClientRequest>,
    active_requests: HashMap<String, LinchpinRequest>,
    linchpin_request_tx: Option<mpsc::Sender<Message>>, // None if no active connection.
    linchpin_response_tx: mpsc::Sender<MessageType>,
    linchpin_response_rx: mpsc::Receiver<MessageType>,
    connection_task: Option<JoinHandle<()>>,
    heartbeat_task: Option<JoinHandle<()>>,
    heartbeat_timeout_secs: u64,
    last_heartbeat: std::time::Instant,
}

pub async fn start(
    config: ClientConfig,
    client_event_tx: mpsc::Sender<ClientEvent>,
    client_request_rx: mpsc::Receiver<ClientRequest>,
) {
    ConnectionManager::new(config, client_event_tx, client_request_rx)
        .init()
        .await;
}

fn send_callback(
    callback: oneshot::Sender<Result<(), LinchpinClientError>>,
    result: Result<(), LinchpinClientError>,
) {
    if let Err(e) = callback.send(result) {
        error!("send_callback: Could not send callback: e={:?}", e);
    }
}

impl ConnectionManager {
    fn new(
        config: ClientConfig,
        client_event_tx: mpsc::Sender<ClientEvent>,
        client_request_rx: mpsc::Receiver<ClientRequest>,
    ) -> ConnectionManager {
        let (linchpin_response_tx, linchpin_response_rx) = mpsc::channel::<MessageType>(16);
        ConnectionManager {
            config: config.clone(),
            client_event_tx,
            client_request_rx,
            active_requests: HashMap::default(),
            linchpin_request_tx: None,
            linchpin_response_tx,
            linchpin_response_rx,
            connection_task: None,
            heartbeat_task: None,
            heartbeat_timeout_secs: DEFAULT_HEARTBEAT_TIMEOUT,
            last_heartbeat: std::time::Instant::now(),
        }
    }

    async fn start_linchpin_connection(&mut self) -> Result<JoinHandle<()>, LinchpinClientError> {
        debug!("start_linchpin_connection: entry");
        let socket = open_socket(&self.config.url, &self.config.sat).await;

        if let Err(e) = socket {
            error!(
                "start_linchpin_connection: Could not create socket: e={:?}",
                e
            );
            return Err(e);
        }

        let (linchpin_request_tx, linchpin_request_rx) = mpsc::channel::<Message>(16);
        self.linchpin_request_tx = Some(linchpin_request_tx);
        let linchpin_response_tx = self.linchpin_response_tx.clone();

        let connection_task = tokio::spawn(async move {
            linchpin_connection::start(socket.unwrap(), linchpin_request_rx, linchpin_response_tx)
                .await;
            debug!("start_linchpin_connection: Exiting connection thread");
        });

        self.emit_event(ClientEvent::State(ClientState::Connected))
            .await;

        Ok(connection_task)
    }

    async fn init(mut self) {
        self.heartbeat_timeout_secs = self
            .config
            .heartbeat_timeout_secs
            .unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT);

        match self.start_linchpin_connection().await {
            Ok(task) => {
                self.connection_task = Some(task);
            }
            Err(e) => {
                self.emit_event(ClientEvent::State(ClientState::Disconnected(e)))
                    .await;
            }
        }

        let tx_for_timer = self.linchpin_response_tx.clone();
        let heartbeat_task = tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(self.heartbeat_timeout_secs));
            loop {
                interval.tick().await;
                if let Err(e) = tx_for_timer.send(MessageType::HeartbeatCheck).await {
                    error!("Failed to send");
                }
            }
        });
        self.heartbeat_task = Some(heartbeat_task);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    from_client = self.client_request_rx.recv() => {
                        if let Err(_) = self.handle_client_request(from_client).await {
                            break;
                        }
                    }
                    from_linchpin = self.linchpin_response_rx.recv() => {
                        self.handle_linchpin_response(from_linchpin).await;
                    }
                }
            }
            debug!("connection_manager: Exiting client_request_rx/linchpin_response_rx thread");
        });
    }

    async fn handle_client_request(
        &mut self,
        message: Option<ClientRequest>,
    ) -> Result<(), LinchpinClientError> {
        debug!("handle_client_request: message={:?}", message);
        if let None = message {
            return Err(LinchpinClientError::IoError);
        }

        match message.unwrap() {
            ClientRequest::Linchpin(request) => {
                self.handle_linchpin_request(request).await;
            }
            ClientRequest::Connection(request) => {
                self.handle_connection_request(request).await;
            }
            ClientRequest::SetSat(data) => {
                self.handle_set_sat_request(data).await;
            }
        }
        Ok(())
    }

    async fn handle_linchpin_request(&mut self, message: LinchpinRequest) {
        match message {
            LinchpinRequest::Subscribe(data) => self.handle_subscribe_request(data).await,
            LinchpinRequest::Unsubscribe(data) => self.handle_unsubscribe_request(data).await,
        }
    }

    async fn handle_linchpin_response(&mut self, message_type: Option<MessageType>) {
        trace!(
            "connection_manager: handle_linchpin_response: message_type={:?}",
            message_type
        );
        if let None = message_type {
            return;
        }

        match message_type.unwrap() {
            MessageType::ListenAck(message) => {
                let active_request = self.active_requests.remove(&message.request_id);
                if let None = active_request {
                    warn!(
                        "handle_linchpin_response: Active request not found: request_id={}",
                        message.request_id
                    );
                    return;
                }

                let active_request = active_request.unwrap();

                if message.status_code != STATUS_CODE_OK {
                    error!(%message.status_code, "handle_linchpin_response: error:");
                    if let Some(callback) = active_request.get_callback() {
                        send_callback(callback, message.as_result());
                    }
                    return;
                }

                match message.operation {
                    Operation::Subscribe => {
                        self.handle_subscribe_response(active_request, message);
                    }
                    Operation::Unsubscribe => {
                        self.handle_unsubscribe_response(active_request, message);
                    }
                    _ => {
                        warn!(
                            "handle_linchpin_response: Unknown: operation={:?}",
                            message.operation
                        );
                        if let Some(callback) = active_request.get_callback() {
                            send_callback(callback, Err(LinchpinClientError::Unknown));
                        }
                    }
                }
            }
            MessageType::ListenMessage(_message) => {
                warn!("handle_linchpin_response: ListenMessage not supported");
            }
            MessageType::NotifyMessage(message) => {
                self.emit_event(ClientEvent::Notify(message)).await;
            }
            MessageType::Heartbeat(_message) => {
                info!("handle_linchpin_response: Heartbeat");
                self.last_heartbeat = Instant::now();

            }
            MessageType::HeartbeatCheck => {
                if self.last_heartbeat.elapsed().as_secs() > self.heartbeat_timeout_secs {
                    debug!("Hearbeat lost, reconnecting");
                    self.handle_reconnect_request().await;
                }
            }
            MessageType::SystemNotification(message) => {
                if message.notification_type == NotificationType::DRAIN_ON {
                    debug!("SystemNotification::DRAIN_ON Reconnecting");
                    self.handle_reconnect_request().await;
                } else {
                    debug!("handle_linchpin_response: SystemNotification");
                }
            }
            MessageType::Disconnected => {
                warn!("handle_linchpin_response: DISCONNECT!");
                if let Some(task) = &self.heartbeat_task {
                    task.abort();
                    self.heartbeat_task = None;
                }
                if let Some(task) = &self.connection_task {
                    task.abort();
                    self.connection_task = None;
                }
                self.emit_event(ClientEvent::State(ClientState::Disconnected(
                    LinchpinClientError::Disconnected,
                )))
                .await;
            }
            _ => {
                debug!("handle_linchpin_response: Unknown");
            }
        }
    }

    async fn handle_connection_request(&mut self, message: ConnectionRequest) {
        match message {
            ConnectionRequest::Disconnect => {
                self.handle_disconnect_request();
            }
            ConnectionRequest::Reconnect => {
                self.handle_reconnect_request().await;
            }
            _ => {
                warn!(
                    "handle_connection_request: Unimplemented: message={:?}",
                    message
                );
            }
        }
    }

    async fn handle_subscribe_request(&mut self, data: SubscribeRequestData) {
        let linchpin_message = Message::new(Operation::Subscribe, data.topic.clone());
        let request_id = linchpin_message.request_id.clone();
        self.active_requests
            .insert(request_id, LinchpinRequest::Subscribe(data));
        self.send_to_linchpin(linchpin_message).await;
    }

    async fn handle_unsubscribe_request(&mut self, data: UnsubscribeRequestData) {
        let linchpin_message = Message::new(Operation::Unsubscribe, data.topic.clone());
        self.active_requests.insert(
            linchpin_message.request_id.clone(),
            LinchpinRequest::Unsubscribe(data),
        );
        self.send_to_linchpin(linchpin_message).await;
    }

    fn handle_disconnect_request(&mut self) {
        if let Some(task) = &self.connection_task {
            task.abort();
            self.connection_task = None;
        }
    }

    async fn handle_reconnect_request(&mut self) {
        if let Some(task) = &self.connection_task {
            task.abort();
            self.connection_task = None;
        }
        match self.start_linchpin_connection().await {
            Ok(task) => {
                self.connection_task = Some(task);
                self.active_requests_on_reconnect().await;
            }
            Err(e) => {
                self.emit_event(ClientEvent::State(ClientState::Disconnected(e)))
                    .await;
            }
        }
    }

    async fn handle_set_sat_request(&mut self, data: SetSatRequestData) {
        self.config.sat = data.sat;
        send_callback(data.callback, Ok(()));
    }

    async fn active_requests_on_reconnect(&mut self) {
        // Reprocess any active requests that were pending when linchpin disconnected.
        let request_ids = Vec::from_iter(self.active_requests.keys().cloned().into_iter());
        for request_id in request_ids {
            if let Some(request) = self.active_requests.remove(&request_id) {
                self.handle_linchpin_request(request).await;
            }
        }
    }

    fn handle_subscribe_response(
        &self,
        active_request: LinchpinRequest,
        _message: ListenAckMessage,
    ) {
        debug!("connection_manager: handle_subscribe_response: entry");
        let data = active_request.as_subscribe_request().unwrap();
        send_callback(data.callback, Ok(()));
    }

    fn handle_unsubscribe_response(
        &mut self,
        active_request: LinchpinRequest,
        _message: ListenAckMessage,
    ) {
        debug!("connection_manager: handle_unsubscribe_response: entry");
        let data = active_request.as_unsubscribe_request().unwrap();
        send_callback(data.callback, Ok(()));
    }

    async fn send_to_linchpin(&mut self, message: Message) {
        debug!(
            "connection_manager: send_to_linchpin: message={:?}",
            message
        );
        match &self.linchpin_request_tx {
            Some(tx) => {
                if let Err(e) = tx.send(message).await {
                    error!(
                        "connection_manager: send_to_linchpin: Could not send request: e={:?}",
                        e
                    );
                }
            }
            None => {
                warn!("connection_manager: send_to_linchpin: Connection not available");
            }
        }
    }

    async fn emit_event(&self, event: ClientEvent) {
        if let Err(e) = self.client_event_tx.send(event).await {
            error!("emit_event: Could not send event: e={:?}", e);
        }
    }
}
