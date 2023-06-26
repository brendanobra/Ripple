use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dpab_core::message::Role;
use dpab_core::model::api_grants::{
    ApiGrantRequest, ApiGrantResponse, ApiName, ApiRequestedGrantType, AuthorizationDisposition,
    AuthorizationOutCome,
};
use dpab_core::model::firebolt::{FireboltPermission, FireboltPermissionRegistry};
use dpab_core::{
    message::{DistributorSession, DpabError, PermissionRequestPayload, PermissionResponse},
    model::permissions::PermissionService,
};
use serde::{Deserialize, Serialize};
use tonic::{async_trait, transport::Channel};
use tracing::{debug, error, info};

use crate::gateway::appsanity_gateway::GrpcClientSession;
use crate::permission_service::app_permissions_service_client::AppPermissionsServiceClient;
use crate::permission_service::{AppKey, EnumeratePermissionsRequest, GetThorTokenRequest};
use crate::util::service_util::create_grpc_client_session;
use crate::util::service_util::decorate_request_with_session;
use cached::proc_macro::cached;

use tonic::Request;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ThorPermissionError {
    ServiceError { context: String },
    TimedOutError,
    IoError,
}

///Union of registry (list of all possible permissions, mapped into legacy badger permissions)
///and a client that calls TPS over gRPC to get actual permissions.

/// On an authorized(firebolt_method,appId,partnerId) call  it will:
/// 1) call TPS authorize(appId,partnerId) (similar to resident app) which enumerates the permissions that
/// are granted to that (appId,partnerId). The permissions are returned as a list of granted TPS permissions, for instance
/// [DATA_zipCode,DATA_timeZone]
/// 2)call permission_registry.firebolt_method_has_permission(firebolt_method, TPS permission) and collect return PermissionStatuses

#[derive(Clone)]
pub struct ThorPermissionService {
    permission_registry: Box<dyn FireboltPermissionRegistry + 'static>,
    grpc_client_session: Arc<Mutex<GrpcClientSession>>,
}
unsafe impl Send for ThorPermissionService {}
unsafe impl Sync for ThorPermissionService {}

#[cfg(test)]
const MOCK_PERMISSION_STATE: &[&str] = &[
    "xrn:superman:capability:strength:super",
    "xrn:superman:capability:light:speed",
    "xrn:superman:capability:navigation:flight",
];

/// Due to various things w/statics and lifetimes, the #[cached] proc macro
/// will only play nice with this function outside of the struct/impl/trait

#[cached(
    key = "String",
    time = 3600,
    convert = r#"{ format!("{}{}", distributor_session.id, app_id) }"#,
    /*
    OTTX-28575 - tell proc macro to not cache if underlying call failed 
    */
    result = true,
)]
pub async fn cached_get_app_permissions(
    channel: Channel,
    distributor_session: &DistributorSession,
    /*
    app_id is only an arg for caching purposes
    */
    app_id: String,
    permission_filters: Vec<String>,
) -> Result<Vec<String>, ThorPermissionError> {
    debug!(
        "thor-permission.cached_get_app_permissions app_id={}, partner_id={}",
        app_id, distributor_session.id
    );

    let mut authorize_client: AppPermissionsServiceClient<_> =
        AppPermissionsServiceClient::with_interceptor(
            channel.to_owned(),
            move |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &distributor_session);
                Ok(req)
            },
        );

    let request = tonic::Request::new(EnumeratePermissionsRequest {
        app_key: Some(AppKey {
            app: app_id.clone(),
            syndication_partner_id: distributor_session.id.clone(),
        }),
        permission_filters,
    });

    let response = tokio::time::timeout(
        Duration::from_secs(5),
        authorize_client.enumerate_permissions(request),
    )
    .await;

    match response {
        Ok(Ok(app_permissions)) => {
            debug!("thor_permission.cached_get_app_permissions received OK response from Server");
            let inner = app_permissions.into_inner();
            Ok(inner.clone().permissions)
        }
        Ok(Err(err)) => {
            error!(
                "thor_permission.cached_get_app_permissions returned error = {:?}",
                err
            );
            Err(ThorPermissionError::ServiceError {
                context: format!("{:?}", err),
            })
        }
        Err(_) => {
            error!("thor_permission.cached_get_app_permissions request TIMED OUT !!!");
            Err(ThorPermissionError::TimedOutError)
        }
    }
}

impl ThorPermissionService {
    #[allow(unused)]
    pub async fn get_app_permissions(
        &self,
        distributor_session: &DistributorSession,
        app_id: String,
        permission_filters: Vec<String>,
    ) -> Result<Vec<String>, ThorPermissionError> {
        debug!("thor_permission.get_app_permissions");
        #[cfg(test)]
        {
            return Ok(MOCK_PERMISSION_STATE
                .to_vec()
                .into_iter()
                .map(|f| String::from(f))
                .collect());
        }
        #[cfg(not(test))]
        {
            let channel = self
                .grpc_client_session
                .lock()
                .unwrap()
                .get_grpc_channel()
                .clone();
            cached_get_app_permissions(channel, distributor_session, app_id, permission_filters)
                .await
        }
    }

    pub async fn get_thor_token(
        &self,
        distributor_session: &DistributorSession,
        app_id: String,
        content_provider: String,
        device_session_id: String,
        app_session_id: String,
    ) -> Result<String, ThorPermissionError> {
        let channel = self
            .grpc_client_session
            .lock()
            .unwrap()
            .get_grpc_channel()
            .clone();

        let mut client: AppPermissionsServiceClient<_> =
            AppPermissionsServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
                decorate_request_with_session(&mut req, &distributor_session);
                Ok(req)
            });

        let perm_req = tonic::Request::new(GetThorTokenRequest {
            app: app_id,
            content_provider: content_provider.clone(),
            device_session_id: device_session_id.clone(),
            app_session_id: app_session_id.clone(),
            token_mode: String::from("untrusted"),
            ttl: 3600,
        });

        match client.get_thor_token(perm_req).await {
            Ok(perm_resp) => {
                debug!("Permission service received OK response from Server");
                let r = perm_resp.into_inner();
                /*
                Save token off to later use
                */
                Ok(r.token)
            }
            Err(err) => {
                error!("Permission service returned an error, err={}", err);
                Err(ThorPermissionError::ServiceError {
                    context: format!("{:?}", err),
                })
            }
        }
    }

    pub fn new_from(
        grpc_client_session: Arc<Mutex<GrpcClientSession>>,
        permission_registry: Box<(dyn FireboltPermissionRegistry + 'static)>,
    ) -> ThorPermissionService {
        ThorPermissionService {
            permission_registry: permission_registry,
            grpc_client_session,
        }
    }

    pub fn new(
        permission_service_uri: String,
        permission_registry: Box<(dyn FireboltPermissionRegistry + 'static)>,
    ) -> ThorPermissionService {
        ThorPermissionService::new_from(
            create_grpc_client_session(permission_service_uri),
            permission_registry,
        )
    }

    ///get All TP permissions that map to an api name
    pub fn firebolt_api_to_tp_api(&self, firebolt_api: ApiName) -> Vec<String> {
        let found: Vec<FireboltPermission> = self
            .permission_registry
            .permissions()
            .into_iter()
            .filter(|perm| perm.contains_api(firebolt_api.to_string()))
            .collect();
        found.into_iter().map(|perm| perm.id).collect()
    }

    pub fn firebolt_cap_to_tp_api(&self, capability: &String, role: &Role) -> Vec<String> {
        let found: Vec<FireboltPermission> = self
            .permission_registry
            .permissions()
            .into_iter()
            .filter(|perm| {
                println!("{} {:?} {}", perm.id, perm.capability, capability);
                perm.contains_capability(capability, role)
            })
            .collect();
        found.into_iter().map(|perm| perm.id).collect()
    }

    pub fn check_if_capability_granted(
        &self,
        capability: String,
        role: Role,
        granted_thor_permissions: Vec<String>,
    ) -> bool {
        let tp_permissions = self.firebolt_cap_to_tp_api(&capability, &role);
        if tp_permissions.len() == 0 {
            false
        } else {
            let allowed_permissions: HashSet<String> =
                granted_thor_permissions.into_iter().collect();
            let requested_permission_aliases: HashSet<String> =
                tp_permissions.into_iter().collect();
            // permission is granted if at least one of the requested permission's aliases is allowed
            let granted = !requested_permission_aliases.is_disjoint(&allowed_permissions);
            if !granted {
                info!(
                    "Permission denied for {:?}, requested aliases={:?}, allowed={:?}",
                    capability, requested_permission_aliases, allowed_permissions
                );
            }
            granted
        }
    }

    pub fn is_app_authorized_for_api(
        &self,
        request: ApiGrantRequest,
        granted_thor_permissions: Vec<String>,
    ) -> ApiGrantResponse {
        /* get list of TP permissions that are mapped to the api_name (could be more than one, but seems unlikely) */
        debug!("processing request {:?}", request);
        let cap_c = request.capability.clone();
        let api_name = request.api_name.clone();
        let tp_permissions = if let Some(capability) = cap_c {
            self.firebolt_cap_to_tp_api(&capability, &Role::Use)
        } else {
            self.firebolt_api_to_tp_api(api_name)
        };

        if tp_permissions.len() == 0 {
            info!("Denied {:?}, app had no thor permissions", request);
            return ApiGrantResponse {
                request: request,
                outcome: AuthorizationOutCome {
                    grant_type: ApiRequestedGrantType::Denied,
                    disposition: AuthorizationDisposition::Denied,
                },
            };
        }
        let allowed_permissions: HashSet<String> = granted_thor_permissions.into_iter().collect();
        let requested_permissions: HashSet<String> = tp_permissions.into_iter().collect();

        let granted_permissions = (&requested_permissions - &allowed_permissions)
            .iter()
            .cloned()
            .into_iter()
            .len()
            > 0;
        /*
        Iterate over grants and see if any of them match the requested grant
        */

        let requested_grant = request.clone().requested_grant;
        if granted_permissions {
            ApiGrantResponse {
                request: request,
                outcome: AuthorizationOutCome {
                    grant_type: requested_grant,
                    disposition: AuthorizationDisposition::Allowed,
                },
            }
        } else {
            info!(
                "Denied {:?}, permissions from thor are {:?}",
                request, allowed_permissions
            );
            ApiGrantResponse {
                request: request,
                outcome: AuthorizationOutCome {
                    grant_type: ApiRequestedGrantType::Denied,
                    disposition: AuthorizationDisposition::Denied,
                },
            }
        }
    }
}

#[async_trait]
impl PermissionService<'static> for ThorPermissionService {
    async fn handle_permission(
        self: Box<Self>,
        request: dpab_core::message::PermissionRequest,
    ) -> Result<PermissionResponse, DpabError> {
        let request_permissions = request.clone();
        let max_retry_count = 3;
        let mut retry_cnt = 0;

        while retry_cnt < max_retry_count {
            retry_cnt += 1;
            let result = self
                .get_app_permissions(
                    &request_permissions.session,
                    request_permissions.app_id.clone(),
                    vec![],
                )
                .await;

            match result {
                Ok(permissions) => {
                    debug!("processing permissions {:?}", permissions.clone());
                    match request.payload {
                        PermissionRequestPayload::Check(p) => {
                            let role = p.clone().role.unwrap_or(Role::Use);
                            let cap = p.get().clone().unwrap_or_default();

                            return Ok(PermissionResponse::Check(
                                self.check_if_capability_granted(cap, role, permissions),
                            ));
                        }
                        PermissionRequestPayload::CheckAll(v_p) => {
                            let mut map: HashMap<String, bool> = HashMap::new();
                            for p in v_p {
                                let role = p.clone().role.unwrap_or(Role::Use);
                                let v = p.get().unwrap_or_default();
                                let granted = self.check_if_capability_granted(
                                    v.clone(),
                                    role,
                                    permissions.clone(),
                                );
                                map.insert(v, granted);
                            }

                            return Ok(PermissionResponse::CheckAllCaps(map));
                        }
                        PermissionRequestPayload::ListCaps => {
                            return Ok(PermissionResponse::List(
                                self.permission_registry
                                    .get_firebolt_caps_from_ref(permissions.clone()),
                            ))
                        }
                        PermissionRequestPayload::ListFireboltPermissions => {
                            return Ok(PermissionResponse::FireboltPermissions(
                                self.permission_registry
                                    .get_firebolt_permissions_from_ref(permissions.clone()),
                            ))
                        }
                        PermissionRequestPayload::ListMethods => {
                            let r = permissions.clone();
                            return Ok(PermissionResponse::List(r));
                        }
                    }
                }
                Err(err) => {
                    if err == ThorPermissionError::TimedOutError {
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }
        Err(DpabError::IoError)
    }
}

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    use super::ThorPermissionService;
    use crate::service::thor_permission::cached_get_app_permissions;
    use crate::util::service_util::create_grpc_client_session;
    use dpab_core::message::DistributorSession;
    use dpab_core::model::api_grants::{ApiGrantRequest, ApiName, ApiRequestedGrantType};
    use dpab_core::model::thor_permission_registry::ThorPermissionRegistry;

    /// Below tests are purely implemented for all the corner cases for permission checks
    /// they have no bearing to current yaml file and the permission service endpoint.
    ///
    mod superman_permission_tests {
        use crate::service::thor_permission::{ThorPermissionService, MOCK_PERMISSION_STATE};
        use dpab_core::{
            message::{
                DistributorSession, PermissionRequest, PermissionRequestParam,
                PermissionRequestPayload, PermissionResponse, Role,
            },
            model::{
                firebolt::{
                    CapabilityRole, CapabilityRoleList, FireboltPermission,
                    FireboltPermissionRegistry,
                },
                permissions::PermissionService,
            },
        };

        fn get_session() -> DistributorSession {
            DistributorSession {
                account_id: "1234".into(),
                device_id: "2345".into(),
                id: "3456".into(),
                token: "4567".into(),
            }
        }

        #[derive(Clone)]
        pub struct SupermanPermissionRegistry;

        impl FireboltPermissionRegistry for SupermanPermissionRegistry {
            fn permissions(&self) -> Vec<FireboltPermission> {
                return MOCK_PERMISSION_STATE
                    .clone()
                    .into_iter()
                    .map(|c| FireboltPermission {
                        capability: CapabilityRoleList {
                            use_caps: vec![String::from(*c)],
                            manage_caps: vec![],
                            provide_caps: vec![],
                        },
                        apis: vec![],
                        description: "".into(),
                        display_name: String::from(*c),
                        id: String::from(*c),
                        provider: "Superman Provider".into(),
                    })
                    .collect::<Vec<FireboltPermission>>();
            }
            fn box_clone(&self) -> Box<dyn FireboltPermissionRegistry> {
                Box::new((*self).clone())
            }
            fn get_firebolt_caps_from_ref(&self, ids: Vec<String>) -> Vec<String> {
                return MOCK_PERMISSION_STATE
                    .clone()
                    .into_iter()
                    .filter(|p| ids.contains(&String::from(**p)))
                    .map(|p| String::from(*p))
                    .collect::<Vec<String>>();
            }

            fn get_firebolt_permissions_from_ref(
                &self,
                ids: Vec<String>,
            ) -> Vec<dpab_core::model::firebolt::CapabilityRole> {
                self.get_firebolt_caps_from_ref(ids)
                    .into_iter()
                    .map(|c| CapabilityRole {
                        cap: c,
                        role: Role::Use,
                    })
                    .collect()
            }
        }

        fn get_request(p: PermissionRequestPayload) -> PermissionRequest {
            PermissionRequest {
                app_id: "superman_app".into(),
                session: get_session(),
                payload: p,
            }
        }

        async fn setup_superman() -> Box<ThorPermissionService> {
            let registry = Box::new(SupermanPermissionRegistry);
            Box::new(ThorPermissionService::new(
                String::from("thor-permission.svc-qa.thor.comcast.com"),
                registry,
            ))
        }

        fn get_check_request(s: String) -> PermissionRequest {
            get_request(PermissionRequestPayload::Check(PermissionRequestParam {
                capability: Some(s),
                method: None,
                role: None,
            }))
        }

        #[tokio::test]
        pub async fn test_success_check() {
            let new_test = setup_superman().await;
            if let Ok(r) = new_test
                .handle_permission(get_check_request(
                    "xrn:superman:capability:navigation:flight".into(),
                ))
                .await
            {
                match r {
                    PermissionResponse::Check(l) => {
                        if !l {
                            panic!("failed")
                        }
                    }
                    _ => panic!("failed"),
                }
            } else {
                panic!("invalid response")
            }
        }

        #[tokio::test]
        pub async fn test_fail_check() {
            let new_test = setup_superman().await;
            if let Ok(r) = new_test
                // You're not that Guy, Pal. Trust Me :)
                .handle_permission(get_check_request(
                    "xrn:superman:capability:navigation:webslinging".into(),
                ))
                .await
            {
                match r {
                    PermissionResponse::Check(l) => {
                        if l {
                            panic!("failed")
                        }
                    }
                    _ => panic!("failed"),
                }
            } else {
                panic!("invalid response")
            }
        }

        #[tokio::test]
        pub async fn test_check_multiple() {
            let new_test = setup_superman().await;
            let r = get_request(PermissionRequestPayload::CheckAll(vec![
                PermissionRequestParam {
                    capability: None,
                    method: Some("xrn:superman:capability:navigation:webslinging".into()),
                    role: None,
                },
                PermissionRequestParam {
                    capability: None,
                    method: Some("xrn:superman:capability:navigation:flight".into()),
                    role: None,
                },
                PermissionRequestParam {
                    capability: None,
                    method: Some("xrn:superman:capability:strength:super".into()),
                    role: None,
                },
            ]));
            if let Ok(r) = new_test.handle_permission(r).await {
                match r {
                    PermissionResponse::CheckAllCaps(l) => {
                        assert_eq!(
                            l.get("xrn:superman:capability:navigation:webslinging")
                                .unwrap(),
                            &false
                        );
                        assert_eq!(
                            l.get("xrn:superman:capability:navigation:flight").unwrap(),
                            &true
                        );
                        assert_eq!(
                            l.get("xrn:superman:capability:strength:super").unwrap(),
                            &true
                        );
                    }
                    _ => panic!("failed"),
                }
            } else {
                panic!("invalid response")
            }
        }

        #[tokio::test]
        pub async fn test_get_list_caps() {
            let new_test = setup_superman().await;
            let r = get_request(PermissionRequestPayload::ListCaps);
            if let Ok(r) = new_test.handle_permission(r).await {
                match r {
                    PermissionResponse::List(caps) => {
                        assert_eq!(caps.len(), 3);
                        assert!(caps.contains(&"xrn:superman:capability:navigation:flight".into()));
                        assert!(!caps
                            .contains(&"xrn:superman:capability:navigation:webslinging".into()));
                    }
                    _ => panic!("failed"),
                }
            } else {
                panic!("invalid response")
            }
        }
    }

    pub async fn test_new() {
        let registry = ThorPermissionRegistry::new();
        let under_test = ThorPermissionService::new(
            String::from("thor-permission.svc-qa.thor.comcast.com"),
            registry,
        );
        let distributor_session = DistributorSession {
            id: String::from("xglobal"),
            token: String::from("token"),
            account_id: String::from("account"),
            device_id: String::from("device_id"),
        };
        let result = under_test
            .get_app_permissions(&distributor_session, String::from("amazonPrime"), vec![])
            .await;
        let fb_api = "Localization.postalCode";
        let _tp_api = under_test
            .firebolt_api_to_tp_api(ApiName::from_str(fb_api).unwrap())
            .get(0)
            .unwrap()
            .clone();

        let grant_request = ApiGrantRequest {
            app_id: String::from("amazonPrime"),
            partner_id: String::from("xglobal"),
            api_name: ApiName::from_str(fb_api).unwrap(),
            requested_grant: ApiRequestedGrantType::ReadOnly,
            capability: None,
        };

        match result {
            Ok(r) => {
                let re = under_test.is_app_authorized_for_api(grant_request, r);
                println!("r={:?}", re);
            }

            Err(e) => panic!("{:?}", e),
        }
    }

    #[tokio::test]
    pub async fn test_app_authorized() {
        let permission_registry = ThorPermissionRegistry::new();
        let test_thor_permission = ThorPermissionService::new(
            String::from("thor-permission.svc-qa.thor.comcast.com"),
            permission_registry,
        );

        let request = ApiGrantRequest {
            app_id: String::from("amazonPrime"),
            partner_id: String::from("xglobal"),
            api_name: ApiName::from_str("account.uid").unwrap(),
            requested_grant: ApiRequestedGrantType::ReadOnly,
            capability: None,
        };

        let mut granted_thor_permissions = Vec::with_capacity(1);
        granted_thor_permissions.push(String::from("true"));

        test_thor_permission.is_app_authorized_for_api(request, granted_thor_permissions);
    }

    #[tokio::test]
    pub async fn test_get_app_permission() {
        let test_session =
            create_grpc_client_session("thor-permission.svc-qa.thor.comcast.com".to_string());

        let channel = test_session.lock().unwrap().get_grpc_channel().clone();

        let distributor_session = DistributorSession {
            id: String::from("id_1"),
            token: String::from("token_permissions"),
            account_id: String::from("account_id_1"),
            device_id: String::from("device_id_1"),
        };

        let app_id = String::from("App_id");

        let mut permission_filters = Vec::with_capacity(1);
        permission_filters.push(String::from("granted"));
        /*
        TODO, this does not really do anything useful, like assert */

        let _ = cached_get_app_permissions(
            channel,
            &distributor_session.clone(),
            app_id,
            permission_filters,
        )
        .await;
    }
}
