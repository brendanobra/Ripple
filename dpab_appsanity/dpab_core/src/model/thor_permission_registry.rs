use crate::model::firebolt::FireboltApi;
use crate::{message::Role, model::badger::BadgerDataField};

use serde::{Deserialize, Serialize};

use super::{
    badger::BadgerPermission,
    firebolt::{
        CapabilityRole, CapabilityRoleList, FireboltPermission, FireboltPermissionRegistry,
    },
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThorPermissionYaml {
    id: String,
    description: String,
    display_name: String,
    badger_data_fields: Option<Vec<String>>,
    firebolt_apis: Option<Vec<String>>,
    hammer_apis: Option<Vec<String>>,
    firebolt_capability: Option<Vec<String>>,
    firebolt_provide_capability: Option<Vec<String>>,
}

impl From<&ThorPermissionYaml> for BadgerPermission {
    fn from(the_yaml: &ThorPermissionYaml) -> Self {
        let badger_data_fields = the_yaml
            .clone()
            .badger_data_fields
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|the_field| BadgerDataField {
                name: the_field.to_string(),
            })
            .collect();

        BadgerPermission {
            provider: String::from("Thor Permission"),
            id: the_yaml.id.clone(),
            description: the_yaml.description.clone(),
            data_fields: badger_data_fields,
        }
    }
}
impl From<&ThorPermissionYaml> for FireboltPermission {
    fn from(the_yaml: &ThorPermissionYaml) -> Self {
        let firebolt_apis = the_yaml
            .clone()
            .firebolt_apis
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|the_field| FireboltApi {
                name: the_field.to_string(),
            })
            .collect();

        FireboltPermission {
            provider: String::from("Thor Permission"),
            id: the_yaml.id.clone(),
            display_name: the_yaml.display_name.clone(),
            description: the_yaml.description.clone(),
            apis: firebolt_apis,
            capability: CapabilityRoleList {
                use_caps: the_yaml.firebolt_capability.clone().unwrap_or_default(),
                provide_caps: the_yaml
                    .firebolt_provide_capability
                    .clone()
                    .unwrap_or_default(),
                manage_caps: vec![],
            },
        }
    }
}
#[derive(Clone)]
pub struct ThorPermissionRegistry {
    yaml_permissions: Vec<ThorPermissionYaml>,
}
/*
Hold the "master" list of permissions, aka
https://github.comcast.com/ottx/thor-permission-registry/blob/develop/permissions.yaml
Provides convenience type conversion/filtering for
badger, hammer and firebolt permissions, which are all unioned in yaml ^^^

*/
impl ThorPermissionRegistry {
    fn new_from(permissions_str: &str) -> Box<Self> {
        /*
        Parse the constant into an array of ThorPermissions
        */

        Box::new(ThorPermissionRegistry {
            yaml_permissions: serde_yaml::from_str(permissions_str).unwrap(),
        })
    }
    pub fn new() -> Box<Self> {
        /*TODO - file path to load from needs to be bound to a cargo config
        to faciliate more flexibility (or possibly use a param to
        enable runtime passing of path to load)
        */
        let permissions_yaml = std::include_str!("thor_permission_registry.yaml");
        ThorPermissionRegistry::new_from(permissions_yaml)
    }
    pub fn badger_permissions(&self) -> Vec<BadgerPermission> {
        self.yaml_permissions
            .iter()
            .filter(|perm| perm.badger_data_fields.is_some())
            .map(|perm| perm.into())
            .collect()
    }
    pub fn firebolt_permissions(&self) -> Vec<FireboltPermission> {
        self.yaml_permissions
            .iter()
            .filter(|perm| perm.firebolt_apis.is_some())
            .map(|perm| perm.into())
            .collect()
    }
}

impl FireboltPermissionRegistry for ThorPermissionRegistry {
    fn permissions(&self) -> Vec<FireboltPermission> {
        self.firebolt_permissions()
    }
    fn box_clone(&self) -> Box<dyn FireboltPermissionRegistry> {
        Box::new((*self).clone())
    }
    fn get_firebolt_caps_from_ref(&self, ids: Vec<String>) -> Vec<String> {
        let r: Vec<Vec<String>> = self
            .yaml_permissions
            .iter()
            .filter(|perm| perm.firebolt_capability.is_some() && ids.contains(&perm.id))
            .map(|perm| perm.firebolt_capability.clone().unwrap())
            .collect();

        let mut result = Vec::new();
        for caps in r {
            result.extend(caps);
        }
        result
    }

    fn get_firebolt_permissions_from_ref(&self, ids: Vec<String>) -> Vec<CapabilityRole> {
        let r: Vec<Vec<CapabilityRole>> = self
            .yaml_permissions
            .iter()
            .filter(|perm| {
                (perm.firebolt_capability.is_some() || perm.firebolt_provide_capability.is_some())
                    && ids.contains(&perm.id)
            })
            .map(|perm| {
                let mut vec = Vec::new();
                if perm.firebolt_capability.is_some() {
                    let caps = (&perm.firebolt_capability).as_ref().unwrap();
                    for c in caps {
                        vec.push(CapabilityRole {
                            cap: c.clone(),
                            role: Role::Use,
                        });
                    }
                }
                if perm.firebolt_provide_capability.is_some() {
                    let caps = (&perm.firebolt_provide_capability).as_ref().unwrap();
                    for c in caps {
                        vec.push(CapabilityRole {
                            cap: c.clone(),
                            role: Role::Provide,
                        });
                    }
                }
                vec
            })
            .collect();

        let mut result = Vec::new();
        for caps in r {
            for c in caps {
                if !result.contains(&c) {
                    result.push(c)
                }
            }
        }
        result
    }
}

#[cfg(test)]
pub mod tests {
    use super::ThorPermissionRegistry;
    use crate::{
        message::Role,
        model::firebolt::{CapabilityRole, FireboltPermissionRegistry},
    };
    #[test]
    pub fn test_load() {
        let under_test = *ThorPermissionRegistry::new();
        println!("total size=={:?}", under_test.yaml_permissions.len());
        println!(
            "badger_permissions={:?}",
            under_test.badger_permissions().len()
        );
        println!(
            "firebolt_permissions={:?}",
            under_test.firebolt_permissions().len()
        );
        // assert_eq!(false,under_test.api_has_permission(String::from("asdf"), String::from("asdf")));
        // assert!(under_test.api_has_permission(String::from("Localization.postalCode"), String::from("DATA_zipCode")));
        // assert!(under_test.api_has_permission(String::from("Device.uid"), String::from("DATA_hashed_ids")));
        // assert!(under_test.api_has_permission(String::from("Account.uid"), String::from("DATA_hashed_ids")));
        // assert!(under_test.api_has_permission(String::from("info.receiverId"), String::from("DATA_receiverId")) == false);
        assert!(!under_test.badger_permissions().is_empty());
        assert!(!under_test.firebolt_permissions().is_empty());
        assert!(!under_test.permissions().is_empty());
    }

    #[test]
    pub fn test_get_firebolt_permissions_from_ref() {
        let reg = ThorPermissionRegistry::new();
        let perms = vec![
            String::from("DATA_receiverId_hashed"),
            String::from("DATA_deviceHash"),
            String::from("API_discovery_onPullPurchasedContent"),
        ];
        let fb_perms = reg.get_firebolt_permissions_from_ref(perms);
        // 3 capabilities given, 2 from DATA_deviceHash, 1 from API_discovery_onPullPurchasedContent
        // DATA_receiverId_hashed has a duplicate capability from DATA_deviceHash. Test that duplicates are removed.
        assert_eq!(fb_perms.len(), 3);
        assert_eq!(
            fb_perms.contains(&CapabilityRole {
                cap: "xrn:firebolt:capability:device:info".into(),
                role: Role::Use
            }),
            true
        );
        assert_eq!(
            fb_perms.contains(&CapabilityRole {
                cap: "xrn:firebolt:capability:device:uid".into(),
                role: Role::Use
            }),
            true
        );
        assert_eq!(
            fb_perms.contains(&CapabilityRole {
                cap: "xrn:firebolt:capability:discovery:purchased-content".into(),
                role: Role::Provide
            }),
            true
        );
    }
}
