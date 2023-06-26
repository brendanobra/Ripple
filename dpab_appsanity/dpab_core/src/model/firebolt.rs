use serde::{Deserialize, Serialize};

use crate::message::Role;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct FireboltApi {
    pub name: String,
}
impl FireboltApi {
    fn is(&self, other_name: String) -> bool {
        self.name.to_lowercase().eq(&other_name.to_lowercase())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapabilityRoleList {
    pub use_caps: Vec<String>,
    pub provide_caps: Vec<String>,
    pub manage_caps: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CapabilityRole {
    pub cap: String,
    pub role: Role,
}

/*
Firebolt permission
*/
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FireboltPermission {
    pub provider: String,
    pub id: String,
    pub description: String,
    pub display_name: String,
    pub apis: Vec<FireboltApi>,
    pub capability: CapabilityRoleList,
}
impl FireboltPermission {
    pub fn contains_api(&self, api_name: String) -> bool {
        let aps: Vec<FireboltApi> = self
            .apis
            .clone()
            .into_iter()
            .filter(|api| api.is(api_name.clone()))
            .collect();
        !aps.is_empty()
    }
    pub fn contains_permission(&self, permission_id: String) -> bool {
        self.id.eq(&permission_id)
    }
    pub fn contains_capability(&self, capability: &String, role: &Role) -> bool {
        let list = match role {
            Role::Use => &self.capability.use_caps,
            Role::Manage => &self.capability.manage_caps,
            Role::Provide => &self.capability.provide_caps,
        };
        list.contains(capability)
    }
}

pub trait FireboltPermissionRegistry: Send {
    /*
    Get canonical list of ALL available permissions from registry */
    fn permissions(&self) -> Vec<FireboltPermission>;
    fn box_clone(&self) -> Box<dyn FireboltPermissionRegistry>;
    fn get_firebolt_caps_from_ref(&self, ids: Vec<String>) -> Vec<String>;
    fn get_firebolt_permissions_from_ref(&self, ids: Vec<String>) -> Vec<CapabilityRole>;
}
impl Clone for Box<dyn FireboltPermissionRegistry> {
    fn clone(&self) -> Box<dyn FireboltPermissionRegistry> {
        self.box_clone()
    }
}
