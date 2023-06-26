use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client_messages::LinchpinClientError;

pub const STATUS_CODE_OK: u32 = 0;
pub const STATUS_CODE_REQUEST_TIMEOUT: u32 = 1;
pub const STATUS_CODE_BAD_REQUEST: u32 = 400;
pub const STATUS_CODE_TOPIC_NOT_FOUND: u32 = 404;
pub const STATUS_CODE_TOO_MANY_SUBSCRIBE_AND_UNSUBSCRIBE_REQUEST: u32 = 429;
pub const STATUS_CODE_INTERNAL_SERVER_ERROR: u32 = 500;
pub const STATUS_CODE_UNKNOWN: u32 = 999;
pub const STATUS_CODE_RETRY_ATTEMPTS_REACHED_LIMIT: u32 = 7;
pub const STATUS_CODE_BROKER_UNAVAILABLE: u32 = 4;
pub const STATUS_CODE_PARTIAL_BROKER_GROUPS_UNAVAILABLE: u32 = 5;
pub const STATUS_CODE_BROKER_IN_SYNC_STATE: u32 = 6;
pub const STATUS_CODE_SUBS_REJECTED_MAX_SUBS_PER_CONNECTION_EXCEEDED: u32 = 431;
pub const STATUS_CODE_SERVICE_UNAVAILABLE: u32 = 503;
pub const STATUS_CODE_OPERATION_NOT_ALLOWED: u32 = 405;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    Subscribe,
    Unsubscribe,
    Publish,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    Disconnected,
    ListenAck(ListenAckMessage),
    ListenMessage(ListenMessage),
    NotifyMessage(NotifyMessage),
    Heartbeat(HeartbeatMessage),
    SystemNotification(DrainMessage),
    HeartbeatCheck,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub request_id: String,
    pub operation: Operation,
    pub topics: Vec<String>,
    pub timestamp: u64,
}

impl Message {
    pub fn new(operation: Operation, topic: String) -> Message {
        Message {
            request_id: Uuid::new_v4().to_string(),
            operation,
            timestamp: get_current_time_ms(),
            topics: vec![topic],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishMessage {
    pub message: Message,
    pub payload: String,
    pub payload_type: String,
}

impl PublishMessage {
    pub fn new(topic: String, payload: String, payload_type: String) -> PublishMessage {
        PublishMessage {
            message: Message::new(Operation::Publish, topic),
            payload,
            payload_type,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////////
// TODO: Nail down what properties are truly optional for all message structs. //
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub address: String,
    pub port: u32,
    pub status: u32,
    pub host_type: String,
    pub dc: String,
    pub server_state: String,
    pub throttle_requests_state: String,
    pub missing_heartbeat: Option<bool>,
    pub host_type_verified: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessage {
    #[serde(rename = "type")]
    pub _type: MessageType,
    pub request_id: String,
    pub sender: Sender,
    pub status_code: u32,
    pub status_message: String,
    pub operation: Operation,
    pub delivery_count: u32,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyMessage {
    pub request_id: String,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub topic: String,
    pub payload: String,
    pub payload_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatMessage {
    pub request_id: String,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub sender: Sender,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainMessage {
    pub request_id: String,
    pub notification_message: String,
    pub notification_type: NotificationType,
    pub message_type: Option<String>, // "SystemNotification"
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum NotificationType {
    DRAIN_ON,
    DRAIN_OFF,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenMessage {
    // TODO: Figure out content
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenAckMessage {
    pub request_id: String,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub sender: Sender,
    pub status_code: u32,
    pub status_message: String,
    pub topics: Vec<String>,
    pub operation: Operation,
    pub delivery_count: Option<u32>,
}

impl ListenAckMessage {
    pub fn as_result(&self) -> Result<(), LinchpinClientError> {
        match self.status_code {
            STATUS_CODE_OK => Ok(()),
            STATUS_CODE_REQUEST_TIMEOUT => Err(LinchpinClientError::Timeout),
            STATUS_CODE_BAD_REQUEST => Err(LinchpinClientError::InvalidRequest),
            STATUS_CODE_TOPIC_NOT_FOUND => Err(LinchpinClientError::NotFound),
            STATUS_CODE_TOO_MANY_SUBSCRIBE_AND_UNSUBSCRIBE_REQUEST => {
                Err(LinchpinClientError::LimitReached)
            }
            STATUS_CODE_INTERNAL_SERVER_ERROR => Err(LinchpinClientError::Unknown),
            STATUS_CODE_UNKNOWN => Err(LinchpinClientError::Unknown),
            STATUS_CODE_RETRY_ATTEMPTS_REACHED_LIMIT => Err(LinchpinClientError::LimitReached),
            STATUS_CODE_BROKER_UNAVAILABLE => Err(LinchpinClientError::Unavailble),
            STATUS_CODE_PARTIAL_BROKER_GROUPS_UNAVAILABLE => Err(LinchpinClientError::Unavailble),
            STATUS_CODE_BROKER_IN_SYNC_STATE => Err(LinchpinClientError::Unknown),
            STATUS_CODE_SUBS_REJECTED_MAX_SUBS_PER_CONNECTION_EXCEEDED => {
                Err(LinchpinClientError::LimitReached)
            }
            STATUS_CODE_SERVICE_UNAVAILABLE => Err(LinchpinClientError::Unavailble),
            STATUS_CODE_OPERATION_NOT_ALLOWED => Err(LinchpinClientError::Unauthorized),
            _ => Err(LinchpinClientError::Unknown),
        }
    }
}

fn get_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct PublishRequestMessage {
    pub publish_message: PublishMessage,
    #[serde(rename = "type")]
    pub _type: Option<MessageType>,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub sender: Option<Sender>,
    pub timeout: u32,
}

impl PublishRequestMessage {
    pub fn new(
        topic: String,
        payload: String,
        payload_type: String,
        _expected_responses: u32,
        _ttl: String,
        timeout: u32,
    ) -> PublishRequestMessage {
        PublishRequestMessage {
            publish_message: PublishMessage {
                message: Message::new(Operation::Publish, topic),
                payload,
                payload_type,
            },
            _type: Some(MessageType::ListenMessage(ListenMessage {})),
            headers: None,
            message_type: None,
            sender: None,
            timeout,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct PublishResponseMessage {
    pub publish_message: PublishMessage,
    #[serde(rename = "type")]
    pub _type: Option<MessageType>,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub sender: Option<Sender>,
    pub timeout: u64,
}

impl PublishResponseMessage {
    pub fn new(
        payload: String,
        payload_type: String,
        request: String,
        _response_token: String,
    ) -> PublishResponseMessage {
        PublishResponseMessage {
            publish_message: PublishMessage {
                message: Message::new(Operation::Publish, request.clone()),
                payload,
                payload_type,
            },
            _type: Some(MessageType::ListenMessage(ListenMessage {})),
            headers: None,
            message_type: None,
            sender: None,
            timeout: get_current_time_ms(),
        }
    }
}
