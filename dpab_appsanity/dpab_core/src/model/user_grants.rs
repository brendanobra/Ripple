use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::message::{DistributorSession, Role};

// use r::ripple::api::permissions::user_grants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantStatus {
    Allowed,
    Denied,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserGrantRequest {
    pub grant_entry: CloudGrantEntry,
    pub dist_session: DistributorSession,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudGrantEntry {
    pub role: Role,
    pub capability: String,
    pub status: GrantStatus,
    pub last_modified_time: Duration, // Duration since Unix epoch
    pub expiry_time: Option<Duration>,
    pub app_name: Option<String>,
}
