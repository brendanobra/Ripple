use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use jsonrpsee::{proc_macros::rpc, core::{RpcResult, server::rpc_module::Methods}};
use ripple_sdk::{extn::{dist::governance::{DataGovernanceChannel, DataGovernanceRequest, DataGovernanceRequestPayload, DataGovernanceRequestType, DataGovernanceResponsePayload}, jsonrpsee::JsonRpseeExtension}, service::{state::ServiceStateMessage, CallContext}, config::manager::ConfigManager, plugin::{RippleExtnMeta, RippleCap, Extensionhelper, ExtnRequest, ExtnResponse}, semver::Version, };
use serde::Deserialize;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Default)]
pub struct XVPPrivacyChannel {
    opt_out: Arc<RwLock<Option<String>>>
}

#[async_trait]
impl DataGovernanceChannel for XVPPrivacyChannel {
    async fn start(
        &self,
        state_tx: Sender<ServiceStateMessage>,
        cm: Box<ConfigManager>,
        mut gov_rx: Receiver<DataGovernanceRequest>,
        helper: Box<dyn Extensionhelper>
    ) {
        loop {
            if let Some(r) = gov_rx.recv().await {
                let callback = r.callback;
                match r.payload {
                    DataGovernanceRequestPayload::Call(c) => {
                        match c {
                            DataGovernanceRequestType::GetOptOut => {
                                let r = self.opt_out.read().unwrap();
                                let s = r.as_ref().unwrap().clone();
                                let _ = callback.send(DataGovernanceResponsePayload::String(s));
                            },
                            DataGovernanceRequestType::SetOptOut(s) => {
                                let mut r = self.opt_out.write().unwrap();
                                let _ = r.insert(s);
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
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


pub struct XvpPrivacyRpcImpl<I> {
    extn_helper:I
}

#[derive(Deserialize, Debug)]
pub struct SetProperty {
    pub value: String,
}

#[rpc(server)]
pub trait XvpPrivacy {
    #[method(name = "privacy.getOptOut")]
    async fn get_opt_out(&self, ctx: CallContext) -> RpcResult<String>;

    #[method(name = "privacy.setOptOut")]
    async fn set_opt_out(&self, ctx: CallContext, value:SetProperty ) -> RpcResult<String>;
}


#[async_trait]
impl XvpPrivacyServer for XvpPrivacyRpcImpl<Box<dyn Extensionhelper >> {
    async fn get_opt_out(&self, ctx: CallContext) -> RpcResult<String>{
        let request = ExtnRequest::DataGovernance(DataGovernanceRequestPayload::Call(DataGovernanceRequestType::GetOptOut));
        if let Ok(response) = self.extn_helper.handle(request).await {
            match response {
                ExtnResponse::DataGovernance(d) => {
                    match d {
                        DataGovernanceResponsePayload::String(s) => return Ok(s),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok("none".into())
    }

    async fn set_opt_out(&self, ctx: CallContext, value:SetProperty ) -> RpcResult<String> {
        println!("{:?}",ctx);
        let request = ExtnRequest::DataGovernance(DataGovernanceRequestPayload::Call(DataGovernanceRequestType::SetOptOut(value.value)));
        if let Ok(response) = self.extn_helper.handle(request).await {
            match response {
                ExtnResponse::DataGovernance(d) => {
                    match d {
                        DataGovernanceResponsePayload::String(s) => return Ok(s),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok("none".into())
    }
}

pub struct XvpPrivacyExtn;

impl JsonRpseeExtension for XvpPrivacyExtn {
    fn get(&self, helper: Box<dyn Extensionhelper>) -> Methods {
        let mut m = Methods::new();
        let _ = m.merge(XvpPrivacyRpcImpl {
            extn_helper: helper
        }.into_rpc());
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
