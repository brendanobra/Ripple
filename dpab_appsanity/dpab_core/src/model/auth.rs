use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::message::{DistributorSession, DpabError, DpabRequest};

use super::{api_grants::ApiName, firebolt::FireboltPermission};

#[derive(Debug, Clone)]
pub enum AuthRequest {
    GetPlatformToken(GetPlatformTokenParams),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetAppPermissionsParams {
    pub app_id: String,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPlatformTokenParams {
    pub app_id: String,
    pub content_provider: String,
    pub device_session_id: String,
    pub app_session_id: String,
    pub dist_session: DistributorSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetAppMethodPermissionParams {
    pub app_id: String,
    pub distributor_session: DistributorSession,
    pub method: ApiName,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppPermissions {
    pub partner_id: String,
    pub permissions: Vec<FireboltPermission>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GetAppMethodPermissionsError {
    ApiAuthenticationFailed,
    ApiAccessGrantFailed,
    ServiceCallFailed { provider: String, context: String },
}

#[async_trait]
pub trait AppPermissionsProvider {
    async fn permissions(&self, app_id: String, partner_id: String) -> AppPermissions;
}

#[async_trait]
pub trait AuthService<'a> {
    async fn get_platform_token(
        self: Box<Self>,
        request: DpabRequest,
        get_platform_token_params: GetPlatformTokenParams,
    ) -> Result<String, DpabError>;
}
