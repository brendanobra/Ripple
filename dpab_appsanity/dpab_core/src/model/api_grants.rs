use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum ApiRequestedGrantType {
    ReadOnly,
    Write,
    Update,
    Denied,
}
impl ApiRequestedGrantType {
    pub fn value(self) -> String {
        match self {
            ApiRequestedGrantType::ReadOnly => "readonly".to_string(),
            ApiRequestedGrantType::Write => "write".to_string(),
            ApiRequestedGrantType::Update => "update".to_string(),
            ApiRequestedGrantType::Denied => "denied".to_string(),
        }
    }
    pub fn from_str(grant_type: String) -> ApiRequestedGrantType {
        match grant_type.as_str() {
            "readonly" => ApiRequestedGrantType::ReadOnly,
            "write" => ApiRequestedGrantType::Write,
            "update" => ApiRequestedGrantType::Update,
            "denied" => ApiRequestedGrantType::Denied,
            _ => ApiRequestedGrantType::Denied,
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum AuthorizationDisposition {
    Allowed,
    Denied,
}
#[derive(Debug, PartialEq, Clone)]
pub struct AuthorizationOutCome {
    pub grant_type: ApiRequestedGrantType,
    pub disposition: AuthorizationDisposition,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ApiName {
    NoApi,
    TimeZone,
    Zipcode,
    AccountUid,
    HouseholdId,
    DeviceUid,
    ReceiverId,
    HashedIds,
    DeviceModel,
    /*
    ToDo, add more */
}

impl FromStr for ApiName {
    type Err = ();
    /*
    Little ugly but has to be done somewhere - map ALL methods, including badger method, into types */
    fn from_str(api_in_str: &str) -> Result<Self, Self::Err> {
        /*remove badger. if present */
        match api_in_str
            .to_lowercase()
            .as_str()
            .replace("badger.", "")
            .as_str()
        {
            /*Firebolt mappings */
            "localization.postalcode" | "info.zipcode" => Ok(ApiName::Zipcode),
            "device.uid" => Ok(ApiName::DeviceUid),
            "account.uid" | "info.householdid" => Ok(ApiName::AccountUid),
            "info.receiverid" => Ok(ApiName::ReceiverId),
            "device.model" => Ok(ApiName::DeviceModel),
            _ => Ok(ApiName::NoApi),
        }
    }
}
impl fmt::Display for ApiName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiName::Zipcode => write!(f, "localization.postalcode"),
            ApiName::NoApi => write!(f, "no.api"),
            ApiName::TimeZone => write!(f, "info.timezone"),
            ApiName::AccountUid => write!(f, "account.uid"),
            ApiName::DeviceUid => write!(f, "localization.postalcode"),
            ApiName::ReceiverId => write!(f, "info.receiverId"),
            ApiName::DeviceModel => write!(f, "device.model"),
            _ => write!(f, "no.api"),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct ApiGrantRequest {
    pub app_id: String,
    pub partner_id: String,
    pub api_name: ApiName,
    pub requested_grant: ApiRequestedGrantType,
    pub capability: Option<String>,
}
#[derive(Debug, PartialEq, Clone)]
pub struct ApiGrantResponse {
    pub request: ApiGrantRequest,
    pub outcome: AuthorizationOutCome,
}
