// If not stated otherwise in this file or this component's license file the
// following copyright and licenses apply:
//
// Copyright 2023 RDK Management
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    RpcModule,
};
use ripple_sdk::{
    api::{
        firebolt::fb_capabilities::{CapEvent, FireboltCap},
        gateway::rpc_gateway_api::CallContext,
        session::{AccountSessionRequest, AccountSessionTokenRequest},
    },
    extn::extn_client_message::ExtnResponse,
    log::error,
};

use crate::{
    firebolt::rpc::RippleRPCProvider,
    state::{cap::cap_state::CapState, platform_state::PlatformState},
    utils::rpc_utils::rpc_err,
};

#[rpc(server)]
pub trait Account {
    #[method(name = "account.session")]
    async fn session(&self, ctx: CallContext, a_t_r: AccountSessionTokenRequest) -> RpcResult<()>;
    #[method(name = "account.id")]
    async fn id_rpc(&self, ctx: CallContext) -> RpcResult<String>;
    #[method(name = "account.uid")]
    async fn uid_rpc(&self, ctx: CallContext) -> RpcResult<String>;
}

#[derive(Debug, Clone)]
pub struct AccountImpl {
    pub platform_state: PlatformState,
}

#[async_trait]
impl AccountServer for AccountImpl {
    async fn session(&self, ctx: CallContext, a_t_r: AccountSessionTokenRequest) -> RpcResult<()> {
        let resp = self
            .platform_state
            .get_client()
            .send_extn_request(AccountSessionRequest::SetAccessToken(a_t_r))
            .await;
        if let Err(_) = resp {
            error!("Error in session {:?}", resp);
            return Err(rpc_err("session error response TBD"));
        }

        // clear the cached distributor session
        self.platform_state
            .session_state
            .clear_session(&ctx.session_id.clone());

        match resp {
            Ok(payload) => match payload.payload.extract().unwrap() {
                ExtnResponse::None(()) => {
                    CapState::emit(
                        &self.platform_state,
                        CapEvent::OnAvailable,
                        FireboltCap::Full("xrn:firebolt:capability:account:session".to_owned()),
                        None,
                    )
                    .await;
                    Ok(())
                }
                _ => {
                    CapState::emit(
                        &self.platform_state,
                        CapEvent::OnUnavailable,
                        FireboltCap::Full("xrn:firebolt:capability:account:session".to_owned()),
                        None,
                    )
                    .await;
                    Err(rpc_err("Provision Status error response TBD"))
                }
            },
            Err(_e) => Err(jsonrpsee::core::Error::Custom(String::from(
                "Provision Status error response TBD",
            ))),
        }
    }

    async fn id_rpc(&self, _ctx: CallContext) -> RpcResult<String> {
        self.id().await
    }

    async fn uid_rpc(&self, _ctx: CallContext) -> RpcResult<String> {
        self.id().await
    }
}
impl AccountImpl {
    pub async fn id(&self) -> RpcResult<String> {
        let response = self
            .platform_state
            .get_client()
            .send_extn_request(AccountSessionRequest::Get)
            .await
            .expect("session");

        if let Some(ExtnResponse::AccountSession(session)) = response.payload.clone().extract() {
            Ok(session.account_id.to_owned())
        } else {
            Err(rpc_err("Account.uid: some failure"))
        }
    }
}

pub struct AccountRPCProvider;
impl RippleRPCProvider<AccountImpl> for AccountRPCProvider {
    fn provide(platform_state: PlatformState) -> RpcModule<AccountImpl> {
        (AccountImpl { platform_state }).into_rpc()
    }
}