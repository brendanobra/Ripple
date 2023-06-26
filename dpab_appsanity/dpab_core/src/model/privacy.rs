use serde::{Deserialize, Serialize};

use crate::message::DistributorSession;

use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PrivacySetting {
    AppDataCollection(String),
    AppEntitlementCollection(String),
    ContinueWatching,
    UnentitledContinueWatching,
    WatchHistory,
    ProductAnalytics,
    Personalization,
    UnentitledPersonalization,
    RemoteDiagnostics,
    PrimaryContentAdTargeting,
    PrimaryBrowseAdTargeting,
    AppContentAdTargeting,
    Acr,
    CameraAnalytics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSetting {
    pub app_id: Option<String>,
    pub value: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivacySettings {
    pub app_data_collection: Option<Vec<AppSetting>>,
    pub app_entitlement_collection: Option<Vec<AppSetting>>,
    pub continue_watching: Option<bool>,
    pub unentitled_continue_watching: Option<bool>,
    pub watch_history: Option<bool>,
    pub product_analytics: Option<bool>,
    pub personalization: Option<bool>,
    pub unentitled_personalization: Option<bool>,
    pub remote_diagnostics: Option<bool>,
    pub primary_content_ad_targeting: Option<bool>,
    pub primary_browse_ad_targeting: Option<bool>,
    pub app_content_ad_targeting: Option<bool>,
    pub acr: Option<bool>,
    pub camera_analytics: Option<bool>,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        PrivacySettings {
            app_data_collection: None,
            app_entitlement_collection: None,
            continue_watching: None,
            unentitled_continue_watching: None,
            watch_history: None,
            product_analytics: None,
            personalization: None,
            unentitled_personalization: None,
            remote_diagnostics: None,
            primary_content_ad_targeting: None,
            primary_browse_ad_targeting: None,
            app_content_ad_targeting: None,
            acr: None,
            camera_analytics: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetPropertyParams {
    pub setting: PrivacySetting,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Clone)]
pub struct SetPropertyParams {
    pub setting: PrivacySetting,
    pub value: bool,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Clone)]
pub enum PrivacyRequest {
    GetProperty(GetPropertyParams),
    GetProperties(DistributorSession),
    SetProperty(SetPropertyParams),
    GetPartnerExclusions(DistributorSession),
}

impl PrivacyRequest {
    pub fn get_session(&self) -> DistributorSession {
        match self {
            PrivacyRequest::GetProperty(params) => params.dist_session.clone(),
            PrivacyRequest::GetProperties(session) => session.clone(),
            PrivacyRequest::SetProperty(params) => params.dist_session.clone(),
            PrivacyRequest::GetPartnerExclusions(session) => session.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ExclusionPolicyData {
    pub data_events: Vec<DataEventType>,
    pub entity_reference: Vec<String>,
    pub derivative_propagation: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ExclusionPolicy {
    pub acr: Option<ExclusionPolicyData>,
    pub app_content_ad_targeting: Option<ExclusionPolicyData>,
    pub business_analytics: Option<ExclusionPolicyData>,
    pub camera_analytics: Option<ExclusionPolicyData>,
    pub continue_watching: Option<ExclusionPolicyData>,
    pub personalization: Option<ExclusionPolicyData>,
    pub primary_browse_ad_targeting: Option<ExclusionPolicyData>,
    pub primary_content_ad_targeting: Option<ExclusionPolicyData>,
    pub product_analytics: Option<ExclusionPolicyData>,
    pub remote_diagnostics: Option<ExclusionPolicyData>,
    pub unentitled_continue_watching: Option<ExclusionPolicyData>,
    pub unentitled_personalization: Option<ExclusionPolicyData>,
    pub watch_history: Option<ExclusionPolicyData>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum DataEventType {
    Watched,
    BusinessIntelligence,
    Unknown,
}

impl FromStr for DataEventType {
    type Err = ();
    fn from_str(input: &str) -> Result<DataEventType, Self::Err> {
        match input {
            "Watch_History" => Ok(DataEventType::Watched),
            _ => Ok(DataEventType::Unknown),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PrivacyResponse {
    Bool(bool),
    Settings(PrivacySettings),
    Exclusions(ExclusionPolicy),
}
