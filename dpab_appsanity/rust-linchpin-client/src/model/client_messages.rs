use super::linchpin_messages::NotifyMessage;
use tokio::sync::{mpsc::error::SendError, oneshot};
use url::ParseError;

pub const DEFAULT_HEARTBEAT_TIMEOUT: u64 = 5 * 60; // Heartbeats * heartbeat interval

#[derive(Debug, PartialEq, Clone)]
pub enum ClientState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected(LinchpinClientError),
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    State(ClientState),
    Notify(NotifyMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinchpinClientError {
    InvalidRequest,
    Disconnected,
    IoError,
    NotFound,
    Timeout,
    Unauthorized,
    LimitReached,
    Unknown,
    Unavailble,
    AlreadySubscribed,
}

impl From<ParseError> for LinchpinClientError {
    fn from(_: ParseError) -> Self {
        LinchpinClientError::InvalidRequest
    }
}

type TungsteniteError = tokio_tungstenite::tungstenite::Error;

impl From<TungsteniteError> for LinchpinClientError {
    fn from(e: TungsteniteError) -> Self {
        match e {
            TungsteniteError::ConnectionClosed => LinchpinClientError::Disconnected,
            TungsteniteError::AlreadyClosed => LinchpinClientError::Disconnected,
            TungsteniteError::Io(_) => LinchpinClientError::Disconnected,
            TungsteniteError::Tls(_) => LinchpinClientError::Disconnected,
            TungsteniteError::Capacity(_) => LinchpinClientError::Disconnected,
            TungsteniteError::SendQueueFull(_) => LinchpinClientError::Disconnected,
            TungsteniteError::Protocol(_) => LinchpinClientError::InvalidRequest,
            TungsteniteError::Utf8 => LinchpinClientError::InvalidRequest,
            TungsteniteError::Url(_) => LinchpinClientError::InvalidRequest,
            TungsteniteError::HttpFormat(_) => LinchpinClientError::InvalidRequest,
            TungsteniteError::Http(response) => match response.status().as_u16() {
                401 => LinchpinClientError::Unauthorized,
                _ => LinchpinClientError::Disconnected,
            },
        }
    }
}

impl From<SendError<ClientRequest>> for LinchpinClientError {
    fn from(_: SendError<ClientRequest>) -> Self {
        LinchpinClientError::Disconnected
    }
}

#[derive(Clone)]
pub struct ClientConfig {
    pub url: String,
    pub sat: String,
    pub initial_reconnect_delay_ms: Option<u64>,
    pub max_reconnect_delay_ms: Option<u64>,
    pub heartbeat_timeout_secs: Option<u64>,
}

#[derive(Debug)]
pub struct SubscribeRequestData {
    pub topic: String,
    pub callback: oneshot::Sender<Result<(), LinchpinClientError>>,
}

#[derive(Debug)]
pub struct UnsubscribeRequestData {
    pub topic: String,
    pub callback: oneshot::Sender<Result<(), LinchpinClientError>>,
}

#[derive(Debug)]
pub struct ConnectRequestData {
    pub callback: oneshot::Sender<Result<(), LinchpinClientError>>,
}

#[derive(Debug)]
pub enum LinchpinRequest {
    Subscribe(SubscribeRequestData),
    Unsubscribe(UnsubscribeRequestData),
}

impl LinchpinRequest {
    pub fn as_subscribe_request(self) -> Option<SubscribeRequestData> {
        if let LinchpinRequest::Subscribe(data) = self {
            return Some(data);
        }
        None
    }

    pub fn as_unsubscribe_request(self) -> Option<UnsubscribeRequestData> {
        if let LinchpinRequest::Unsubscribe(data) = self {
            return Some(data);
        }
        None
    }

    pub fn get_callback(self) -> Option<oneshot::Sender<Result<(), LinchpinClientError>>> {
        match self {
            LinchpinRequest::Subscribe(data) => Some(data.callback),
            LinchpinRequest::Unsubscribe(data) => Some(data.callback),
        }
    }
}

#[derive(Debug)]
pub enum ConnectionRequest {
    Connect(ConnectRequestData),
    Disconnect,
    Reconnect,
}

#[derive(Debug)]
pub struct SetSatRequestData {
    pub sat: String,
    pub callback: oneshot::Sender<Result<(), LinchpinClientError>>,
}

#[derive(Debug)]
pub enum ClientRequest {
    Linchpin(LinchpinRequest),
    Connection(ConnectionRequest),
    SetSat(SetSatRequestData),
}
