use async_trait::async_trait;
use jsonrpsee::{proc_macros::rpc, core::{RpcResult, server::rpc_module::Methods}};
use ripple_sdk::{extn::{dist::governance::{DataGovernanceChannel, DataGovernanceRequest}, jsonrpsee::JsonRpseeExtension}, service::{state::ServiceStateMessage, CallContext}, config::manager::ConfigManager, plugin::{RippleExtnMeta, RippleCap, Extensionhelper}, semver::Version, };
use serde::Deserialize;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct XVPPrivacyChannel;

#[async_trait]
impl DataGovernanceChannel for XVPPrivacyChannel {
    async fn start(
        &self,
        state_tx: Sender<ServiceStateMessage>,
        cm: Box<ConfigManager>,
        dab_rx: Receiver<DataGovernanceRequest>,
    ) {

    }
}

impl RippleExtnMeta for XVPPrivacyChannel {
    fn cap(&self) -> ripple_sdk::plugin::RippleCap {
        RippleCap::get_channel(ripple_sdk::plugin::RippleExtnClass::DataGovernance, "xvp".into(), None)
    }

    fn require(&self) -> Version {
        Version::new(1, 1, 0)
    }
}


pub struct XvpPrivacyRpcImpl;

#[derive(Deserialize, Debug)]
pub struct SetBoolProperty {
    pub value: String,
}

#[rpc(server)]
pub trait XvpPrivacy {
    #[method(name = "privacy.getOptOut")]
    async fn get_opt_out(&self, ctx: CallContext) -> RpcResult<String>;

    #[method(name = "privacy.setOptOut")]
    async fn set_opt_out(&self, ctx: CallContext, value:SetBoolProperty ) -> RpcResult<String>;
}


#[async_trait]
impl XvpPrivacyServer for XvpPrivacyRpcImpl {
    async fn get_opt_out(&self, ctx: CallContext) -> RpcResult<String>{
        Ok("some_value".into())
    }

    async fn set_opt_out(&self, ctx: CallContext, value:SetBoolProperty ) -> RpcResult<String> {
        Ok("some_value".into())
    }
}

pub struct XvpPrivacyExtn;

impl JsonRpseeExtension for XvpPrivacyExtn {
    fn get(&self, helper: Box<dyn Extensionhelper>) -> Methods {
        let mut m = Methods::new();
        let _ = m.merge(XvpPrivacyRpcImpl.into_rpc());
        m
    }
}

impl RippleExtnMeta for XvpPrivacyExtn {
    fn require(&self) -> Version {
        Version::new(1, 1, 0)
    }
    
    fn cap(&self) -> RippleCap {
        RippleCap::get_extn(ripple_sdk::plugin::RippleExtnClass::Jsonrpsee, "privacy".into(), Some("xvp".into()))
    }
}
