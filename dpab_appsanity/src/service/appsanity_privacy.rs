use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use dpab_core::message::Role;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dpab_core::model::user_grants::{CloudGrantEntry, GrantStatus, UserGrantRequest};
use dpab_core::{
    gateway::DpabDelegate,
    message::{
        DistributorSession, DpabError, DpabRequest, DpabRequestPayload, DpabResponse,
        DpabResponsePayload,
    },
    model::privacy::{
        AppSetting, DataEventType, ExclusionPolicy, ExclusionPolicyData, GetPropertyParams,
        PrivacyRequest, PrivacyResponse, PrivacySetting, PrivacySettings, SetPropertyParams,
    },
};

use hyper::body::HttpBody;
use hyper::header::CONTENT_TYPE;
use hyper::http::{HeaderValue, Request};
use hyper::{Body, Client, Method};
use hyper_tls::HttpsConnector;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::Sender as OneShotSender;
use tonic::async_trait;
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    auth::AddAuthorizationLayer, classify::StatusInRangeAsFailures,
    decompression::DecompressionLayer, set_header::SetRequestHeaderLayer, trace::TraceLayer,
};
use tracing::{debug, error, warn};
use url::{ParseError, Url};

type Callback = Option<OneShotSender<DpabResponse>>;

const ENTITY_REFERENCE_PREFIX: &'static str = "xrn:xvp:application:";
const OWNER_REFERENCE_PREFIX: &'static str = "xrn:xcal:subscriber:account:";

enum EssRequest {
    Get(String),
    GetAll,
    Set(String, bool),
    Delete(String),
    DeleteAll,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EssSettingData {
    pub allowed: Option<bool>,
    pub expiration: Option<String>,
    pub owner_reference: Option<String>,
    pub entity_reference: Option<String>,
    #[serde(skip_serializing)]
    pub updated: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum ValueType {
    Many(Vec<EssSettingData>),
    Single(EssSettingData),
}
impl TryFrom<Value> for ValueType {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value.is_array() {
            let mut list: Vec<EssSettingData> = Vec::new();
            for v in value.as_array().unwrap() {
                debug!("About to convert: {:?}", v);
                let setting = serde_json::from_value::<EssSettingData>(v.clone()).map_err(|d| {
                    debug!("error: {:?}", d);
                    "Unable to convert value to ess settings struct"
                });
                //TODO: Have to decide on what to be done if we cant unwrap safely.
                list.push(setting.unwrap());
            }
            return Ok(Self::Many(list));
        } else if value.is_object() {
            let setting = serde_json::from_value::<EssSettingData>(value).map_err(|e| {
                println!("Error: {:?}", e);
                "Unable to convert value to single ess struct"
            })?;
            return Ok(Self::Single(setting));
        } else {
            return Err("Unable to convert");
        }
    }
}

impl EssSettingData {
    pub fn new(
        allowed: Option<bool>,
        expiration: Option<String>,
        owner_reference: Option<String>,
        entity_reference: Option<String>,
        updated: Option<String>,
    ) -> Self {
        EssSettingData {
            allowed,
            expiration,
            owner_reference,
            entity_reference,
            updated,
        }
    }

    pub fn as_app_id(&self) -> Option<String> {
        match &self.entity_reference {
            Some(reference) => parse_entity_reference(reference),
            None => None,
        }
    }

    pub fn get_timestamp_str_from_duration(duration: Duration) -> String {
        let datetime: DateTime<Utc> = (UNIX_EPOCH + duration).into();
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

pub fn parse_entity_reference(reference: &String) -> Option<String> {
    if reference.starts_with(ENTITY_REFERENCE_PREFIX) {
        match reference.get(ENTITY_REFERENCE_PREFIX.len()..) {
            Some(s) => Some(s.to_string()),
            None => None,
        }
    } else {
        warn!(%reference, "as_app_id: Entity reference does not conform to XVP URN expectations");
        return Some(reference.clone());
    }
}

impl Default for EssSettingData {
    fn default() -> Self {
        EssSettingData {
            allowed: None,
            expiration: None,
            owner_reference: None,
            entity_reference: None,
            updated: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(rename = "xcal:appDataCollection")]
    app_data_collection: Option<Vec<EssSetting>>,
    #[serde(rename = "xcal:appEntitlementCollection")]
    app_entitlement_collection: Option<Vec<EssSetting>>,
    #[serde(rename = "xcal:continueWatching")]
    continue_watching: Option<EssSetting>,
    #[serde(rename = "xcal:unentitledContinueWatching")]
    unentitled_continue_watching: Option<EssSetting>,
    #[serde(rename = "xcal:watchHistory")]
    watch_history: Option<EssSetting>,
    #[serde(rename = "xcal:productAnalytics")]
    product_analytics: Option<EssSetting>,
    #[serde(rename = "xcal:personalization")]
    personalization: Option<EssSetting>,
    #[serde(rename = "xcal:unentitledPersonalization")]
    unentitled_personalization: Option<EssSetting>,
    #[serde(rename = "xcal:remoteDiagnostics")]
    remote_diagnostics: Option<EssSetting>,
    #[serde(rename = "xcal:primaryContentAdTargeting")]
    primary_content_ad_targeting: Option<EssSetting>,
    #[serde(rename = "xcal:primaryBrowseAdTargeting")]
    primary_browse_ad_targeting: Option<EssSetting>,
    #[serde(rename = "xcal:appContentAdTargeting")]
    app_content_ad_targeting: Option<EssSetting>,
    #[serde(rename = "xcal:acr")]
    acr: Option<EssSetting>,
    #[serde(rename = "xcal:cameraAnalytics")]
    camera_analytics: Option<EssSetting>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EssGetResponseBody {
    partner_id: String,
    account_id: String,
    settings: Settings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawEssGetResponseBody {
    partner_id: String,
    account_id: String,
    settings: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapRole {
    pub role: Role,
    pub cap: String,
}

impl RawEssGetResponseBody {
    fn date_str_to_std_duration_from_epoch(timestamp: &str) -> Option<Duration> {
        let res_datetime = Utc.datetime_from_str(timestamp, "%Y-%m-%dT%H:%M:%SZ");
        if res_datetime.is_err() {
            error!("Passed date time is not confirming to expected format {timestamp}");
            return None;
        }
        let datetime = res_datetime.unwrap();
        let native_datetime = datetime.naive_utc();
        let epoch_datetime = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
        let res_std_dur = DateTime::<Utc>::from_utc(native_datetime, Utc)
            .signed_duration_since(epoch_datetime)
            .to_std();
        if res_std_dur.is_err() {
            error!("Unable to get duration since epoch");
            return None;
        } else {
            debug!("Converted {timestamp} to sec: {:?}", res_std_dur);
            Some(res_std_dur.unwrap())
        }
    }
    fn get_cloud_entry(settings: &EssSettingData, cap_role: &CapRole) -> CloudGrantEntry {
        let cloud_entry = CloudGrantEntry {
            role: cap_role.role.clone(),
            capability: cap_role.cap.to_owned(),
            status: match settings.allowed {
                Some(allowed) => {
                    if allowed {
                        GrantStatus::Allowed
                    } else {
                        GrantStatus::Denied
                    }
                }
                _ => GrantStatus::Denied,
            },
            last_modified_time: match settings.updated.as_ref() {
                Some(timestamp_str) => Self::date_str_to_std_duration_from_epoch(timestamp_str)
                    .unwrap_or(Duration::new(0, 0)),
                None => Duration::new(0, 0),
            },
            app_name: settings
                .entity_reference
                .as_ref()
                .map(|reference| parse_entity_reference(reference).unwrap_or(reference.to_owned())),
            expiry_time: settings.expiration.as_ref().map(|timestamp_str| {
                Self::date_str_to_std_duration_from_epoch(timestamp_str)
                    .unwrap_or(Duration::new(0, 0))
            }),
        };
        cloud_entry
    }

    pub fn get_grants(
        &self,
        user_grants_mapping: &HashMap<String, CapRole>,
    ) -> Vec<CloudGrantEntry> {
        let mut cloud_grant_entries: Vec<CloudGrantEntry> = Vec::new();
        let ess_settings = &self.settings;
        for (k, v) in ess_settings {
            debug!("key: {k}");
            let cap_role_opt = user_grants_mapping.get(k);
            if cap_role_opt.is_none() {
                continue;
            }
            let cap_role = cap_role_opt.unwrap();
            let vt: Result<ValueType, &str> = v.clone().try_into();
            debug!("{k}: {:?}\n \n", vt);
            if let Ok(v) = vt {
                match v {
                    ValueType::Single(entry) => {
                        cloud_grant_entries.push(Self::get_cloud_entry(&entry, &cap_role))
                    }
                    ValueType::Many(entries) => {
                        for entry in entries {
                            cloud_grant_entries.push(Self::get_cloud_entry(&entry, &cap_role))
                        }
                    }
                }
            }
        }
        cloud_grant_entries
    }
}
impl EssGetResponseBody {
    pub fn get_setting(&self, setting: PrivacySetting) -> Option<EssSetting> {
        fn get_app_setting(
            app_id: String,
            settings: Option<Vec<EssSetting>>,
        ) -> Option<EssSetting> {
            if let None = settings {
                return None;
            }
            for setting in settings.unwrap() {
                let id = setting.data.as_app_id();
                if id.is_none() {
                    continue;
                }
                if id.unwrap().eq(&app_id) {
                    return Some(setting);
                }
            }
            None
        }

        match setting {
            PrivacySetting::AppDataCollection(app_id) => {
                get_app_setting(app_id, self.settings.app_data_collection.clone())
            }
            PrivacySetting::AppEntitlementCollection(app_id) => {
                get_app_setting(app_id, self.settings.app_entitlement_collection.clone())
            }
            PrivacySetting::ContinueWatching => self.settings.continue_watching.clone(),
            PrivacySetting::UnentitledContinueWatching => {
                self.settings.unentitled_continue_watching.clone()
            }
            PrivacySetting::WatchHistory => self.settings.watch_history.clone(),
            PrivacySetting::ProductAnalytics => self.settings.product_analytics.clone(),
            PrivacySetting::Personalization => self.settings.personalization.clone(),
            PrivacySetting::UnentitledPersonalization => {
                self.settings.unentitled_personalization.clone()
            }
            PrivacySetting::RemoteDiagnostics => self.settings.remote_diagnostics.clone(),
            PrivacySetting::PrimaryContentAdTargeting => {
                self.settings.primary_content_ad_targeting.clone()
            }
            PrivacySetting::PrimaryBrowseAdTargeting => {
                self.settings.primary_browse_ad_targeting.clone()
            }
            PrivacySetting::AppContentAdTargeting => self.settings.app_content_ad_targeting.clone(),
            PrivacySetting::Acr => self.settings.acr.clone(),
            PrivacySetting::CameraAnalytics => self.settings.camera_analytics.clone(),
        }
    }

    pub fn get_settings(&self) -> PrivacySettings {
        fn get_allowed(setting: &Option<EssSetting>) -> Option<bool> {
            match setting {
                Some(s) => s.data.allowed,
                None => None,
            }
        }

        fn get_allowed_apps(settings: &Option<Vec<EssSetting>>) -> Option<Vec<AppSetting>> {
            if let None = settings {
                return None;
            }

            let mut app_settings = Vec::new();

            for setting in settings.clone().unwrap() {
                app_settings.push(AppSetting {
                    app_id: setting.data.as_app_id(),
                    value: setting.data.allowed,
                });
            }

            if app_settings.is_empty() {
                return None;
            }

            Some(app_settings)
        }

        PrivacySettings {
            app_data_collection: get_allowed_apps(&self.settings.app_data_collection),
            app_entitlement_collection: get_allowed_apps(&self.settings.app_entitlement_collection),
            continue_watching: get_allowed(&self.settings.continue_watching),
            unentitled_continue_watching: get_allowed(&self.settings.unentitled_continue_watching),
            watch_history: get_allowed(&self.settings.watch_history),
            product_analytics: get_allowed(&self.settings.product_analytics),
            personalization: get_allowed(&self.settings.personalization),
            unentitled_personalization: get_allowed(&self.settings.unentitled_personalization),
            remote_diagnostics: get_allowed(&self.settings.remote_diagnostics),
            primary_content_ad_targeting: get_allowed(&self.settings.primary_content_ad_targeting),
            primary_browse_ad_targeting: get_allowed(&self.settings.primary_browse_ad_targeting),
            app_content_ad_targeting: get_allowed(&self.settings.app_content_ad_targeting),
            acr: get_allowed(&self.settings.acr),
            camera_analytics: get_allowed(&self.settings.camera_analytics),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EssSetting {
    #[serde(skip)]
    pub name: &'static str,
    #[serde(flatten)]
    pub data: EssSettingData,
}

impl EssSetting {
    pub fn new(
        privacy_setting: PrivacySetting,
        value: Option<bool>,
        expiration: Option<String>,
        owner_reference: Option<String>,
        entity_reference: Option<String>,
        updated: Option<String>,
    ) -> EssSetting {
        let data = EssSettingData::new(
            value,
            expiration,
            owner_reference,
            entity_reference,
            updated,
        );
        let name = match privacy_setting {
            PrivacySetting::AppDataCollection(_) => "xcal:appDataCollection",
            PrivacySetting::AppEntitlementCollection(_) => "xcal:appEntitlementCollection",
            PrivacySetting::ContinueWatching => "xcal:continueWatching",
            PrivacySetting::UnentitledContinueWatching => "xcal:unentitledContinueWatching",
            PrivacySetting::WatchHistory => "xcal:watchHistory",
            PrivacySetting::ProductAnalytics => "xcal:productAnalytics",
            PrivacySetting::Personalization => "xcal:personalization",
            PrivacySetting::UnentitledPersonalization => "xcal:unentitledPersonalization",
            PrivacySetting::RemoteDiagnostics => "xcal:remoteDiagnostics",
            PrivacySetting::PrimaryContentAdTargeting => "xcal:primaryContentAdTargeting",
            PrivacySetting::PrimaryBrowseAdTargeting => "xcal:primaryBrowseAdTargeting",
            PrivacySetting::AppContentAdTargeting => "xcal:appContentAdTargeting",
            PrivacySetting::Acr => "xcal:acr",
            PrivacySetting::CameraAnalytics => "xcal:cameraAnalytics",
        };

        EssSetting { name, data }
    }

    pub fn to_body(&self) -> String {
        format!(
            "{{\"{}\": {}}}",
            self.name,
            serde_json::to_string(&self.data).unwrap()
        )
    }
}

impl Default for EssSetting {
    fn default() -> Self {
        EssSetting {
            name: "",
            data: EssSettingData::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ExclusionData {
    #[serde(rename = "dataEvents")]
    data_events: Vec<String>,
    #[serde(rename = "entityReference")]
    entity_reference: Vec<String>,
    #[serde(rename = "derivativePropagation")]
    derivative_propagation: bool,
}

// XXX: use genrics <T> for struct Settings
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Exclusions<T> {
    #[serde(rename = "xcal:acr")]
    acr: Option<T>,
    #[serde(rename = "xcal:appContentAdTargeting")]
    app_content_ad_targeting: Option<T>,
    #[serde(rename = "xcal:businessAnalytics")]
    business_analytics: Option<ExclusionData>,
    #[serde(rename = "xcal:cameraAnalytics")]
    camera_analytics: Option<T>,
    #[serde(rename = "xcal:continueWatching")]
    continue_watching: Option<T>,
    #[serde(rename = "xcal:personalization")]
    personalization: Option<T>,
    #[serde(rename = "xcal:primaryBrowseAdTargeting")]
    primary_browse_ad_targeting: Option<T>,
    #[serde(rename = "xcal:primaryContentAdTargeting")]
    primary_content_ad_targeting: Option<T>,
    #[serde(rename = "xcal:productAnalytics")]
    product_analytics: Option<T>,
    #[serde(rename = "xcal:remoteDiagnostics")]
    remote_diagnostics: Option<T>,
    #[serde(rename = "xcal:unentitledContinueWatching")]
    unentitled_continue_watching: Option<T>,
    #[serde(rename = "xcal:unentitledPersonalization")]
    unentitled_personalization: Option<T>,
    #[serde(rename = "xcal:watchHistory")]
    watch_history: Option<T>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ExclusionGetResponseBody {
    partner_id: String,
    #[serde(rename = "exclusionPolicy")]
    exclusions: Exclusions<ExclusionData>,
}

impl ExclusionGetResponseBody {
    pub fn get_exclusions(&self) -> ExclusionPolicy {
        ExclusionPolicy {
            acr: self.get_data(self.exclusions.acr.clone()),
            app_content_ad_targeting: self
                .get_data(self.exclusions.app_content_ad_targeting.clone()),
            business_analytics: self.get_data(self.exclusions.business_analytics.clone()),
            camera_analytics: self.get_data(self.exclusions.camera_analytics.clone()),
            continue_watching: self.get_data(self.exclusions.continue_watching.clone()),
            personalization: self.get_data(self.exclusions.personalization.clone()),
            primary_browse_ad_targeting: self
                .get_data(self.exclusions.primary_browse_ad_targeting.clone()),
            primary_content_ad_targeting: self
                .get_data(self.exclusions.primary_content_ad_targeting.clone()),
            product_analytics: self.get_data(self.exclusions.product_analytics.clone()),
            remote_diagnostics: self.get_data(self.exclusions.remote_diagnostics.clone()),
            unentitled_continue_watching: self
                .get_data(self.exclusions.unentitled_continue_watching.clone()),
            unentitled_personalization: self
                .get_data(self.exclusions.unentitled_personalization.clone()),
            watch_history: self.get_data(self.exclusions.watch_history.clone()),
        }
    }
    pub fn get_data(&self, excl_data: Option<ExclusionData>) -> Option<ExclusionPolicyData> {
        match excl_data {
            None => None,
            Some(data) => Some(ExclusionPolicyData {
                data_events: get_data_events(data.data_events),
                entity_reference: get_enitity_reference(data.entity_reference),
                derivative_propagation: data.derivative_propagation,
            }),
        }
    }
}

pub fn get_data_events(de_list: Vec<String>) -> Vec<DataEventType> {
    let mut vec = Vec::new();
    for de in de_list {
        tracing::debug!("events string: {:?}", de);
        vec.push(DataEventType::from_str(&de).unwrap());
    }
    vec
}

pub fn get_enitity_reference(er_list: Vec<String>) -> Vec<String> {
    let mut vec = Vec::new();
    for er in er_list {
        vec.push(parse_entity_reference(&er).unwrap_or(er));
    }
    vec
}

struct EssUri {
    endpoint: String,
    partner_id: String,
    account_id: String,
    client_id: String,
    setting: Option<EssSetting>,
    entity: Option<String>,
    show_expired: Option<bool>,
}

impl EssUri {
    pub fn new(
        endpoint: String,
        partner_id: String,
        account_id: String,
        setting: Option<EssSetting>,
        entity: Option<String>,
        show_expired: Option<bool>,
    ) -> Self {
        EssUri {
            endpoint,
            partner_id,
            account_id,
            client_id: "ripple".into(),
            setting,
            entity,
            show_expired,
        }
    }

    pub fn into_privacy_settings_url(&self) -> Result<String, DpabError> {
        let mut url = Url::parse(&self.endpoint)?;
        url.path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("v1")
            .push("partners")
            .push(&self.partner_id)
            .push("accounts")
            .push(&self.account_id)
            .push("privacySettings");

        url.query_pairs_mut()
            .append_pair("clientId", &self.client_id);

        if let Some(setting) = &self.setting {
            url.query_pairs_mut()
                .append_pair("settingFilter", setting.name);
        }

        if let Some(entity) = &self.entity {
            url.query_pairs_mut()
                .append_pair("entityReferenceFilter", entity);
        }

        if let Some(se) = &self.show_expired {
            let show_expired = match se {
                true => "true",
                false => "false",
            };
            url.query_pairs_mut()
                .append_pair("showExpired", show_expired);
        }

        Ok(url.into_string())
    }

    pub fn into_usage_data_exclusion_url(&self) -> Result<String, DpabError> {
        let mut url = Url::parse(&self.endpoint)?;
        url.path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("v1")
            .push("partners")
            .push(&self.partner_id)
            .push("privacySettings")
            .push("policy")
            .push("usageDataExclusions");

        url.query_pairs_mut()
            .append_pair("clientId", &self.client_id);

        if let Some(setting) = &self.setting {
            url.query_pairs_mut()
                .append_pair("settingFilter", setting.name);
        }

        if let Some(entity) = &self.entity {
            url.query_pairs_mut()
                .append_pair("entityReferenceFilter", entity);
        }

        Ok(url.into_string())
    }
}

pub struct PrivacyService {
    endpoint: String,
    dist_session: DistributorSession,
    user_grants_cloud_mapping: HashMap<String, CapRole>,
}

#[async_trait]
impl DpabDelegate for PrivacyService {
    async fn handle(&mut self, request: DpabRequest) {
        match request.payload {
            DpabRequestPayload::Privacy(privacy_request) => {
                let result = match privacy_request {
                    PrivacyRequest::GetProperty(params) => self.get_property(params).await,
                    PrivacyRequest::GetProperties(session) => self.get_properties(session).await,
                    PrivacyRequest::SetProperty(params) => self.set_property(params).await,
                    PrivacyRequest::GetPartnerExclusions(session) => {
                        self.get_partner_exclusions(session).await
                    }
                };
                if let Some(cb) = request.callback {
                    cb.send(result).ok();
                }
            }
            DpabRequestPayload::UserGrants(user_grant_entry) => {
                self.set_user_grant(&user_grant_entry).await;
            }
            _ => {
                error!("handle: Unexpected payload: {:?}", request.payload);
            }
        }
    }
}

impl PrivacyService {
    pub fn get_user_grants_mapping(cloud_firebolt_mapping: &Value) -> HashMap<String, CapRole> {
        if !cloud_firebolt_mapping.is_object() {
            // Without mapping information no need to reach for the clouds. Simple return nothing.
            debug!("cloud mapping not present so returning no user grants");
            return HashMap::new();
        }
        let cloud_mapping = cloud_firebolt_mapping.as_object().unwrap();
        if cloud_mapping.get("user_grants").is_none() {
            // Without user grants mapping information also we cant do anything so no need to reach for the clouds.
            // Simply return nothing.
            debug!("cloud mapping present but does not contain user grants mapping so returning empty list");
            return HashMap::new();
        }
        let user_grants_mapping_val = cloud_mapping.get("user_grants").unwrap();
        if !user_grants_mapping_val.is_object() {
            // Seems not a proper config, so returning Nothing
            debug!("cloud mapping and user grants present but user grants is not an object so returning empty list");
            return HashMap::new();
        }
        let user_grants_mapping_res =
            serde_json::from_value::<HashMap<String, CapRole>>(user_grants_mapping_val.clone());
        if let Err(_) = user_grants_mapping_res {
            debug!("could not convert user grants config to useful struct so returning empty list");
            return HashMap::new();
        }
        user_grants_mapping_res.unwrap()
    }
    pub fn new(
        endpoint: String,
        dist_session: DistributorSession,
        firebolt_cloud_mapping: &Value,
    ) -> PrivacyService {
        let user_grants_cloud_mapping = Self::get_user_grants_mapping(firebolt_cloud_mapping);
        PrivacyService {
            endpoint,
            dist_session,
            user_grants_cloud_mapping,
        }
    }

    async fn get_property(&mut self, params: GetPropertyParams) -> DpabResponse {
        let entity = match params.setting.clone() {
            PrivacySetting::AppDataCollection(app_id) => Some(app_id),
            PrivacySetting::AppEntitlementCollection(app_id) => Some(app_id),
            _ => None,
        };
        let uri = EssUri::new(
            self.endpoint.clone(),
            params.dist_session.id.clone(),
            params.dist_session.account_id.clone(),
            Some(EssSetting::new(
                params.setting.clone(),
                None,
                None,
                None,
                None,
                None,
            )),
            entity,
            Some(false),
        );
        let body_string = self
            .ess_request(
                Method::GET,
                uri.into_privacy_settings_url()?,
                params.dist_session.token.clone(),
                String::default(),
            )
            .await?;
        let resp_body: Result<EssGetResponseBody, serde_json::Error> =
            serde_json::from_str(&body_string);
        if let Ok(r) = resp_body {
            if let Some(setting) = r.get_setting(params.setting) {
                return DpabResponse::Ok(DpabResponsePayload::Privacy(PrivacyResponse::Bool(
                    setting.data.allowed.unwrap(),
                )));
            }
        }

        Err(DpabError::ServiceError)
    }

    async fn get_raw_ess_response(
        &self,
        session: &DistributorSession,
    ) -> Result<RawEssGetResponseBody, DpabError> {
        let uri = EssUri::new(
            self.endpoint.clone(),
            session.id.clone(),
            session.account_id.clone(),
            None,
            None,
            Some(false),
        );
        let body_string = self
            .ess_request(
                Method::GET,
                uri.into_privacy_settings_url()?,
                session.token.clone(),
                String::default(),
            )
            .await?;
        debug!("Received raw response: {:?}", body_string);
        let resp_body: Result<RawEssGetResponseBody, serde_json::Error> =
            serde_json::from_str(&body_string);
        resp_body.map_err(|e| {
            error!("RawEssGetResponseBody parse error {:?}", e);
            DpabError::ServiceError
        })
    }

    async fn get_ess_response(
        &mut self,
        session: DistributorSession,
    ) -> Result<EssGetResponseBody, DpabError> {
        let uri = EssUri::new(
            self.endpoint.clone(),
            session.id.clone(),
            session.account_id.clone(),
            None,
            None,
            Some(false),
        );
        let body_string = self
            .ess_request(
                Method::GET,
                uri.into_privacy_settings_url()?,
                session.token.clone(),
                String::default(),
            )
            .await?;
        debug!("Received raw response: {:?}", body_string);
        let resp_body: Result<EssGetResponseBody, serde_json::Error> =
            serde_json::from_str(&body_string);
        resp_body.map_err(|e| {
            error!("EssGetResponseBody parse error {:?}", e);
            DpabError::ServiceError
        })
    }
    pub async fn get_properties(&mut self, session: DistributorSession) -> DpabResponse {
        let resp_body = self.get_ess_response(session).await;
        if let Ok(r) = resp_body {
            return DpabResponse::Ok(DpabResponsePayload::Privacy(PrivacyResponse::Settings(
                r.get_settings(),
            )));
        }
        Err(DpabError::ServiceError)
    }

    pub async fn get_user_grants(&self) -> DpabResponse {
        let resp_body = self.get_raw_ess_response(&self.dist_session).await;
        debug!("Received ESS response body: {:?}", resp_body);
        if let Ok(resp) = resp_body {
            return DpabResponse::Ok(DpabResponsePayload::UserGrants(
                resp.get_grants(&self.user_grants_cloud_mapping),
            ));
        }
        Err(DpabError::ServiceError)
    }

    async fn get_partner_exclusions(&mut self, session: DistributorSession) -> DpabResponse {
        let uri = EssUri::new(
            self.endpoint.clone(),
            session.id.clone(),
            session.account_id.clone(),
            None,
            None,
            Some(false),
        );
        let response = self
            .ess_request(
                Method::GET,
                uri.into_usage_data_exclusion_url()?,
                session.token.clone(),
                String::default(),
            )
            .await;
        match response {
            Ok(mut body_string) => {
                let resp_body: Result<ExclusionGetResponseBody, serde_json::Error> =
                    serde_json::from_str(&body_string);
                if let Ok(exclusions_obj) = resp_body {
                    return DpabResponse::Ok(DpabResponsePayload::Privacy(
                        PrivacyResponse::Exclusions(exclusions_obj.get_exclusions()),
                    ));
                }
            }
            Err(e) => match e {
                not_data_found => {
                    return DpabResponse::Ok(DpabResponsePayload::Privacy(
                        PrivacyResponse::Exclusions(ExclusionPolicy::default()),
                    ));
                }
            },
        }
        Err(DpabError::ServiceError)
    }

    async fn set_user_grant(&self, params: &UserGrantRequest) -> DpabResponse {
        let grant_entry = params.grant_entry.clone();
        let dist_session = params.dist_session.clone();
        let entity = grant_entry
            .app_name
            .clone()
            .map(|app_name| format!("{}{}", ENTITY_REFERENCE_PREFIX, app_name));

        // let entity = params.
        let owner_reference = format!(
            "{}{}",
            OWNER_REFERENCE_PREFIX, params.dist_session.account_id
        );
        // let property_name = CapRole::
        let mut property_name = None;
        for (k, v) in &self.user_grants_cloud_mapping {
            if v.cap.eq(&params.grant_entry.capability) && v.role == params.grant_entry.role {
                property_name = Some(k.clone());
                break;
            }
        }
        if property_name.is_none() {
            error!(
                "Unable to find property for cap: {} with role: {:?}",
                params.grant_entry.capability, params.grant_entry.role
            );
            return Err(DpabError::NotDataFound);
        }
        let ess_settings_data = EssSettingData {
            allowed: Some(match params.grant_entry.status {
                GrantStatus::Allowed => true,
                GrantStatus::Denied => false,
            }),
            expiration: params
                .grant_entry
                .expiry_time
                .map(|duration| EssSettingData::get_timestamp_str_from_duration(duration)),
            owner_reference: Some(owner_reference),
            entity_reference: entity.clone(),
            updated: None,
        };
        let body = format!(
            "{{\"{}\": [{}]}}",
            property_name.unwrap(),
            serde_json::to_string(&ess_settings_data).unwrap()
        );
        debug!("Formed body for user grants: {:?}", body);
        let uri = EssUri::new(
            self.endpoint.clone(),
            params.dist_session.id.clone(),
            params.dist_session.account_id.clone(),
            None,
            entity,
            None,
        );

        self.ess_request(
            Method::PUT,
            uri.into_privacy_settings_url()?,
            params.dist_session.token.clone(),
            body,
        )
        .await?;
        DpabResponse::Ok(DpabResponsePayload::None)
    }
    async fn set_property(&mut self, params: SetPropertyParams) -> DpabResponse {
        let entity = match params.setting.clone() {
            PrivacySetting::AppDataCollection(app_id) => {
                Some(format!("{}{}", ENTITY_REFERENCE_PREFIX, app_id))
            }
            PrivacySetting::AppEntitlementCollection(app_id) => {
                Some(format!("{}{}", ENTITY_REFERENCE_PREFIX, app_id))
            }
            _ => None,
        };
        let owner_reference = format!(
            "{}{}",
            OWNER_REFERENCE_PREFIX, params.dist_session.account_id
        );
        let setting = EssSetting::new(
            params.setting.clone(),
            Some(params.value),
            None,
            Some(owner_reference),
            None,
            None,
        );
        let body = setting.to_body();
        let uri = EssUri::new(
            self.endpoint.clone(),
            params.dist_session.id.clone(),
            params.dist_session.account_id.clone(),
            None,
            entity,
            None,
        );

        self.ess_request(
            Method::PUT,
            uri.into_privacy_settings_url()?,
            params.dist_session.token.clone(),
            body,
        )
        .await?;
        DpabResponse::Ok(DpabResponsePayload::None)
    }

    async fn ess_request(
        &self,
        method: Method,
        url: String,
        token: String,
        body: String,
    ) -> Result<String, DpabError> {
        let hyper_client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

        let mut client = ServiceBuilder::new()
            .layer(TraceLayer::new(
                StatusInRangeAsFailures::new(400..=599).into_make_classifier(),
            ))
            .layer(
                TraceLayer::new_for_http()
                    .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG)),
            )
            .layer(SetRequestHeaderLayer::overriding(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=UTF-8"),
            ))
            .layer(AddAuthorizationLayer::bearer(&token))
            .layer(DecompressionLayer::new())
            .layer(TraceLayer::new_for_http())
            .service(hyper_client);

        let req = Request::builder()
            .uri(url.clone())
            .method(method.clone())
            .body(Body::from(body.clone()));

        debug!(
            "ess_request: req={:?}, url={}, method={}, body={}, token={}",
            req,
            url,
            method.as_str(),
            body,
            token
        );

        if let Err(_) = req {
            error!("Could not compose ESS request");
            return Err(DpabError::ServiceError);
        }

        let request = req.unwrap();

        // TODO: What's the default response timeout for tower? Do we need to configure?
        let send_response = client.ready().await.unwrap().call(request).await;
        if let Err(e) = send_response {
            error!("error sending to ESS={:?}", e);
            return Err(DpabError::ServiceError);
        }
        let mut response = send_response.unwrap();
        if response.status() == 200 {
            if let Ok(body_bytes) = hyper::body::to_bytes(response.into_body()).await {
                if let Ok(body_string) = String::from_utf8(body_bytes.to_vec()) {
                    return Ok(body_string);
                }
            }
        } else {
            while let Some(chunk) = response.body_mut().data().await {
                error!("{:?}", &chunk);
            }
            error!("ESS returned failure: status={:?}", response.status());

            return if response.status() == 404 {
                Err(DpabError::NotDataFound)
            } else {
                Err(DpabError::ServiceError)
            };
        }
        Err(DpabError::ServiceError)
    }
}
