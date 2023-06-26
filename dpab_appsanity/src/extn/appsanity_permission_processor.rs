use dpab_core::{
    message::{DistributorSession, PermissionRequestPayload},
    model::permissions::PermissionService,
};
use ripple_sdk::{
    api::{
        distributor::distributor_permissions::PermissionRequest,
        firebolt::fb_capabilities::{CapabilityRole, FireboltCap, FireboltPermission},
        session::AccountSession,
    },
    async_trait::async_trait,
    extn::{
        client::{
            extn_client::ExtnClient,
            extn_processor::{
                DefaultExtnStreamer, ExtnRequestProcessor, ExtnStreamProcessor, ExtnStreamer,
            },
        },
        extn_client_message::{ExtnMessage, ExtnResponse},
    },
    log::info,
    utils::error::RippleError,
};
use tokio::sync::mpsc;
use tracing::debug;

use crate::service::thor_permission::ThorPermissionService;

#[derive(Clone)]
pub struct PermissionState {
    client: ExtnClient,
    tps: Box<ThorPermissionService>,
}

pub struct DistributorPermissionProcessor {
    state: PermissionState,
    streamer: DefaultExtnStreamer,
}

fn into_distributor_session(sess: AccountSession) -> DistributorSession {
    DistributorSession {
        id: sess.id,
        token: sess.token,
        account_id: sess.account_id,
        device_id: sess.device_id,
    }
}

fn into_cap_role(role: dpab_core::message::Role) -> CapabilityRole {
    match role {
        dpab_core::message::Role::Use => CapabilityRole::Use,
        dpab_core::message::Role::Manage => CapabilityRole::Manage,
        dpab_core::message::Role::Provide => CapabilityRole::Provide,
    }
}

impl DistributorPermissionProcessor {
    pub fn new(
        client: ExtnClient,
        tps: Box<ThorPermissionService>,
    ) -> DistributorPermissionProcessor {
        DistributorPermissionProcessor {
            state: PermissionState { client, tps },
            streamer: DefaultExtnStreamer::new(),
        }
    }

    async fn process_request(
        mut state: <DistributorPermissionProcessor as ExtnStreamProcessor>::STATE,
        msg: ExtnMessage,
        extracted_message: <DistributorPermissionProcessor as ExtnStreamProcessor>::VALUE,
    ) -> bool {
        debug!("Getting permissions for {}", extracted_message.app_id);
        let req = dpab_core::message::PermissionRequest {
            app_id: extracted_message.app_id.clone(),
            session: into_distributor_session(extracted_message.session.clone()),
            payload: PermissionRequestPayload::ListFireboltPermissions,
        };
        let perm_res = state.tps.handle_permission(req).await;
        if let Err(_) = perm_res {
            return Self::handle_error(state.client.clone(), msg, RippleError::ExtnError).await;
        }
        match perm_res.unwrap() {
            dpab_core::message::PermissionResponse::FireboltPermissions(v) => {
                let perms = v
                    .iter()
                    .map(|cr| FireboltPermission {
                        cap: FireboltCap::Full(cr.cap.clone()),
                        role: into_cap_role(cr.role.clone()),
                    })
                    .collect();
                info!(
                    "{} has firebolt permissions {:?}",
                    extracted_message.app_id, perms
                );
                let resp = ExtnResponse::Permission(perms);
                state.client.respond(msg, resp).await.ok();
            }
            _ => {
                return Self::handle_error(state.client.clone(), msg, RippleError::InvalidOutput)
                    .await
            }
        }
        true
    }
}

impl ExtnStreamProcessor for DistributorPermissionProcessor {
    type STATE = PermissionState;
    type VALUE = PermissionRequest;

    fn get_state(&self) -> Self::STATE {
        self.state.clone()
    }

    fn receiver(&mut self) -> mpsc::Receiver<ExtnMessage> {
        self.streamer.receiver()
    }

    fn sender(&self) -> mpsc::Sender<ExtnMessage> {
        self.streamer.sender()
    }
}

#[async_trait]
impl ExtnRequestProcessor for DistributorPermissionProcessor {
    fn get_client(&self) -> ExtnClient {
        self.state.client.clone()
    }

    async fn process_request(state: Self::STATE, msg: ExtnMessage, val: Self::VALUE) -> bool {
        DistributorPermissionProcessor::process_request(state, msg, val).await
    }
}
