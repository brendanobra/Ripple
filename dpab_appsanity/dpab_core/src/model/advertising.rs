use crate::message::{DistributorSession, DpabRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AdvertisingRequest {
    GetAdInitObject(AdInitObjectRequestParams),
    GetAdIdObject(AdIdRequestParams),
    ResetAdIdentifier(DistributorSession),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdInitObjectRequestParams {
    pub privacy_data: HashMap<String, String>,
    pub environment: String,
    pub durable_app_id: String,
    pub app_version: String,
    pub distributor_app_id: String,
    pub device_ad_attributes: HashMap<String, String>,
    pub coppa: bool,
    pub authentication_entity: String,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdInitObjectResponse {
    pub ad_server_url: String,
    pub ad_server_url_template: String,
    pub ad_network_id: String,
    pub ad_profile_id: String,
    pub ad_site_section_id: String,
    pub ad_opt_out: bool,
    pub privacy_data: String,
    pub ifa_value: String,
    pub ifa: String,
    pub app_name: String,
    pub app_bundle_id: String,
    pub app_version: String,
    pub distributor_app_id: String,
    pub device_ad_attributes: String,
    pub coppa: String,
    pub authentication_entity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdIdRequestParams {
    pub privacy_data: HashMap<String, String>,
    pub app_id: String,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdIdResponse {
    pub ifa: String,
    pub ifa_type: String,
    pub lmt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionParams {
    pub dist_session: DistributorSession,
}

#[async_trait]
pub trait AdvertisingService {
    async fn get_ad_init_object(
        self: Box<Self>,
        request: DpabRequest,
        params: AdInitObjectRequestParams,
    );
    async fn get_ad_identifier(self: Box<Self>, request: DpabRequest, params: AdIdRequestParams);
    async fn reset_ad_identifier(self: Box<Self>, request: DpabRequest, params: DistributorSession);
}
