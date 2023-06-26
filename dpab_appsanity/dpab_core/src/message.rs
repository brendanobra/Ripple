use crate::model::{
    advertising::{AdIdResponse, AdInitObjectResponse, AdvertisingRequest},
    apps::{AppMetadata, AppsRequest},
    auth::{AppPermissions, AuthRequest},
    discovery::{
        ContentAccessResponse, DiscoveryAccountLinkRequest, DiscoveryRequest,
        EntitlementsAccountLinkResponse, LaunchPadAccountLinkResponse,
        MediaEventsAccountLinkResponse,
    },
    firebolt::CapabilityRole,
    metrics::{AppBehavioralMetric, BadgerMetrics, DeviceMetricsContext},
    privacy::{PrivacyRequest, PrivacyResponse},
    secure_storage::{SecureStorageRequest, SecureStorageResponse},
    sync_and_monitor::SyncAndMonitorRequest,
    user_grants::{CloudGrantEntry, UserGrantRequest},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::oneshot::{error::RecvError, Sender as OneShotSender};
use tracing::{error, span, trace};
use url::ParseError;
#[derive(Debug)]
pub struct DpabRequest {
    pub payload: DpabRequestPayload,
    pub callback: Option<OneShotSender<DpabResponse>>,
    pub parent_span: Option<span::Span>,
}

impl DpabRequest {
    pub fn respond(self, response: DpabResponse) -> Result<(), DpabResponse> {
        if let Some(cb) = self.callback {
            return cb.send(response);
        }
        Ok(())
    }

    pub fn respond_and_log(self, response: DpabResponse) {
        let result = self.respond(response);
        match result {
            Ok(_) => trace!("Successfully responded to dpab request"),
            Err(_) => error!("Failed to respond to dpab request"),
        };
    }
}

#[derive(Debug, Clone)]
pub enum DpabRequestPayload {
    Advertising(AdvertisingRequest),
    Auth(AuthRequest),
    AppMetric(
        Option<DeviceMetricsContext>,
        AppBehavioralMetric,
        DistributorSession,
    ),
    BadgerMetric(
        Option<DeviceMetricsContext>,
        BadgerMetrics,
        DistributorSession,
    ),
    Permission(PermissionRequest),
    AccountLink(DiscoveryAccountLinkRequest),
    Discovery(DiscoveryRequest),
    Privacy(PrivacyRequest),
    SecureStorage(SecureStorageRequest),
    Apps(AppsRequest),
    UserGrants(UserGrantRequest),
    SyncAndMonitor(SyncAndMonitorRequest),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallContext {
    pub app_id: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    Use,
    Manage,
    Provide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestParam {
    pub capability: Option<String>,
    pub method: Option<String>,
    pub role: Option<Role>,
}

impl PermissionRequestParam {
    pub fn is_cap(&self) -> bool {
        self.capability.is_some()
    }
    pub fn is_method(&self) -> bool {
        self.method.is_some()
    }
    pub fn is_valid(&self) -> bool {
        self.capability.is_some() || self.method.is_some()
    }
    pub fn has_role(&self) -> bool {
        self.role.is_some()
    }
    pub fn get(self) -> Option<String> {
        if self.capability.is_some() {
            Some(self.capability.unwrap())
        } else if self.method.is_some() {
            Some(self.method.unwrap())
        } else {
            None
        }
    }
    pub fn contains(self, list: Vec<String>) -> bool {
        if let Some(v) = self.get() {
            list.contains(&v)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub app_id: String,
    pub session: DistributorSession,
    pub payload: PermissionRequestPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionRequestPayload {
    ListCaps,
    ListMethods,
    ListFireboltPermissions,
    Check(PermissionRequestParam),
    CheckAll(Vec<PermissionRequestParam>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionResponse {
    FireboltPermissions(Vec<CapabilityRole>),
    List(Vec<String>),
    Check(bool),
    CheckAllMethods(HashMap<String, bool>),
    CheckAllCaps(HashMap<String, bool>),
}

impl DpabRequestPayload {
    pub fn as_advertising_request(&self) -> Option<AdvertisingRequest> {
        match self {
            DpabRequestPayload::Advertising(req) => Some(req.clone()),
            _ => None,
        }
    }
    pub fn as_auth_request(&self) -> Option<AuthRequest> {
        match self {
            DpabRequestPayload::Auth(req) => Some(req.clone()),
            _ => None,
        }
    }

    pub fn as_permission_request(&self) -> Option<PermissionRequest> {
        match self {
            DpabRequestPayload::Permission(p) => Some(p.clone()),
            _ => None,
        }
    }
    pub fn as_account_link_request(&self) -> Option<DiscoveryAccountLinkRequest> {
        match self {
            DpabRequestPayload::AccountLink(req) => Some(req.clone()),
            _ => None,
        }
    }
    pub fn as_secure_storage_request(&self) -> Option<SecureStorageRequest> {
        match self {
            DpabRequestPayload::SecureStorage(req) => Some(req.clone()),
            _ => None,
        }
    }

    pub fn as_sync_and_monitor_request(&self) -> Option<SyncAndMonitorRequest> {
        match self {
            DpabRequestPayload::SyncAndMonitor(req) => Some(req.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PermissionServiceError {
    pub provider: String,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DpabError {
    ServiceError,
    IoError,
    PermissionServiceError(PermissionServiceError),
    Exists,
    NotDataFound,
}

impl From<ParseError> for DpabError {
    fn from(_: ParseError) -> Self {
        DpabError::IoError
    }
}

pub type DpabResponse = Result<DpabResponsePayload, DpabError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DpabResponsePayload {
    JsonValue(Value),
    String(String),
    None,
    AppPermissions(AppPermissions),
    AdInitObject(AdInitObjectResponse),
    AdIdObject(AdIdResponse),
    Permission(PermissionResponse),
    EntitlementsAccountLink(EntitlementsAccountLinkResponse),
    MediaEventsAccountLink(MediaEventsAccountLinkResponse),
    LaunchPadAccountLink(LaunchPadAccountLinkResponse),
    ContentAccess(ContentAccessResponse),
    Privacy(PrivacyResponse),
    UserGrants(Vec<CloudGrantEntry>),
    SecureStorage(SecureStorageResponse),
    AppsUpdate(Vec<AppMetadata>),
}

/// Checks multiple levels of result to see if there was an error
pub fn is_err(res: &Result<Result<DpabResponsePayload, DpabError>, RecvError>) -> bool {
    if res.is_err() {
        return true;
    }
    if res.as_ref().unwrap().is_err() {
        return true;
    }
    false
}

impl DpabResponsePayload {
    pub fn as_string(&self) -> Option<String> {
        match self {
            DpabResponsePayload::String(resp) => Some(resp.clone()),
            DpabResponsePayload::JsonValue(resp) => Some(String::from(resp.as_str().unwrap())),
            DpabResponsePayload::None | _ => None,
        }
    }

    pub fn as_value(&self) -> Option<Value> {
        match self {
            DpabResponsePayload::JsonValue(resp) => Some(resp.clone()),
            DpabResponsePayload::String(resp) => Some(serde_json::from_str(resp).unwrap()),
            DpabResponsePayload::None | _ => None,
        }
    }

    pub fn as_permission_response(&self) -> Option<&PermissionResponse> {
        match self {
            DpabResponsePayload::Permission(p) => Some(p),
            _ => None,
        }
    }
    pub fn as_secure_storage_response(&self) -> Option<&SecureStorageResponse> {
        match self {
            DpabResponsePayload::SecureStorage(p) => Some(p),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DistributorSession {
    pub id: String,
    pub token: String,
    pub account_id: String,
    pub device_id: String,
}

impl DistributorSession {
    pub fn new(
        id: Option<String>,
        token: Option<String>,
        account_id: Option<String>,
        device_id: Option<String>,
    ) -> DistributorSession {
        DistributorSession {
            id: id.unwrap_or_default(),
            token: token.unwrap_or_default(),
            account_id: account_id.unwrap_or_default(),
            device_id: device_id.unwrap_or_default(),
        }
    }
}
