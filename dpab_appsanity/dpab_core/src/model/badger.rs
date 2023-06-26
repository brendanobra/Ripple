use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BadgerDataField {
    pub name: String,
}

pub struct BadgerPermission {
    pub provider: String,
    pub id: String,
    pub description: String,
    pub data_fields: Vec<BadgerDataField>,
}
