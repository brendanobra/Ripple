use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::message::DistributorSession;

#[derive(Debug, Clone)]
pub enum AppsRequest {
    OnAppsUpdate(Sender<AppsUpdate>),
    RefreshSession(DistributorSession),
    CheckForUpdates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub title: String,
    pub version: String,
    pub uri: String,
    pub data: Option<String>,
}

impl AppMetadata {
    pub fn new(
        id: String,
        title: String,
        version: String,
        uri: String,
        data: Option<String>,
    ) -> AppMetadata {
        AppMetadata {
            id,
            title,
            version,
            uri,
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppsUpdate {
    pub apps: Vec<AppMetadata>,
}

impl AppsUpdate {
    pub fn new(apps: Vec<AppMetadata>) -> AppsUpdate {
        AppsUpdate { apps }
    }
}
