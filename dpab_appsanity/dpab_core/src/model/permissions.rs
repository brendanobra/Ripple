use async_trait::async_trait;
use tracing::info;

use crate::message::{DistributorSession, DpabError, PermissionRequest, PermissionResponse};

pub trait PermissionRegistry<T> {
    fn permissions(&self) -> Vec<T>;
}
#[async_trait]
pub trait PermissionsManager: Send + Sync {
    async fn is_api_authorized(
        &self,
        distributor_session: DistributorSession,
        app_id: String,
        method: String,
    ) -> bool;
}
#[derive(Clone)]
pub struct GrantingPermissionsManager {}
impl GrantingPermissionsManager {
    pub fn new() -> impl PermissionsManager + Send + Sync {
        GrantingPermissionsManager {}
    }
}
///
/// This is a convenience implementation that will be used
/// when dpab_appsanity is not enabled.
///
#[async_trait]
impl PermissionsManager for GrantingPermissionsManager {
    async fn is_api_authorized(
        &self,
        _distributor_session: DistributorSession,
        app_id: String,
        method: String,
    ) -> bool {
        info!("local authing {}.{} as granted", app_id, method);
        true
    }
}

#[async_trait]
pub trait PermissionService<'a> {
    async fn handle_permission(
        self: Box<Self>,
        request: PermissionRequest,
    ) -> Result<PermissionResponse, DpabError>;
}
