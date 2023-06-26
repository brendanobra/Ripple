use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HammerApi {
    name: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HammerPermission {
    pub provider: String,
    pub id: String,
    pub name: String,
    pub api: HammerApi,
}
