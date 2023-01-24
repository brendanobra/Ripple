use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use jsonrpsee::{
    core::{server::rpc_module::Methods, RpcResult},
    proc_macros::rpc,
};
use ripple_sdk::{
    config::manager::ConfigManager,
    extn::{
        dist::governance::{
            DataGovernanceChannel, DataGovernanceRequest, DataGovernanceRequestPayload,
            DataGovernanceRequestType, DataGovernanceResponsePayload,
        },
        jsonrpsee::JsonRpseeExtension,
    },
    plugin::{Extensionhelper, ExtnRequest, ExtnResponse, RippleCap, RippleExtnMeta},
    semver::Version,
    service::{state::ServiceStateMessage, CallContext},
};
use serde::Deserialize;
use tokio::{sync::mpsc::{Receiver, Sender}, runtime::Runtime};

#[derive(Clone, Default)]
pub struct XVPPrivacyChannel {
    opt_out: Arc<RwLock<Option<String>>>,
}

impl DataGovernanceChannel for XVPPrivacyChannel {
    fn start(
        &self,
        state_tx: Box<Sender<ServiceStateMessage>>,
        cm: Box<ConfigManager>,
        mut gov_rx: Box<Receiver<DataGovernanceRequest>>,
        helper: Box<dyn Extensionhelper>,
    ) {
        let runtime = Runtime::new().unwrap();
        let opt_out = self.opt_out.clone();
        runtime.spawn(async move {
            loop {
                if let Some(r) = gov_rx.recv().await {
                    let callback = r.callback;
                    match r.payload {
                        DataGovernanceRequestPayload::Call(c) => match c {
                            DataGovernanceRequestType::GetOptOut => {
                                let r = opt_out.read().unwrap();
                                let s = r.as_ref().unwrap().clone();
                                let _ = callback.send(DataGovernanceResponsePayload::String(s));
                            }
                            DataGovernanceRequestType::SetOptOut(s) => {
                                let mut r = opt_out.write().unwrap();
                                let _ = r.insert(s);
                                let _ = callback.send(DataGovernanceResponsePayload::Bool(true));
                            }
                        },
                        _ => {}
                    }
                }
            }
        });
    }
}

impl RippleExtnMeta for XVPPrivacyChannel {
    fn cap(&self) -> ripple_sdk::plugin::RippleCap {
        RippleCap::get_channel(
            ripple_sdk::plugin::RippleExtnClass::DataGovernance,
            "xvp".into(),
            None,
        )
    }

    fn require(&self) -> Version {
        Version::new(1, 1, 0)
    }
}

pub struct XvpPrivacyRpcImpl<I> {
    extn_helper: I,
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
    async fn set_opt_out(&self, ctx: CallContext, value: SetProperty) -> RpcResult<String>;
}

#[async_trait]
impl XvpPrivacyServer for XvpPrivacyRpcImpl<Box<dyn Extensionhelper>> {
    async fn get_opt_out(&self, ctx: CallContext) -> RpcResult<String> {
        let request = ExtnRequest::DataGovernance(DataGovernanceRequestPayload::Call(
            DataGovernanceRequestType::GetOptOut,
        ));
        if let Ok(response) = self.extn_helper.handle(request).await {
            match response {
                ExtnResponse::DataGovernance(d) => match d {
                    DataGovernanceResponsePayload::String(s) => return Ok(s),
                    _ => {}
                },
                _ => {}
            }
        }

        Ok("none".into())
    }

    async fn set_opt_out(&self, ctx: CallContext, value: SetProperty) -> RpcResult<String> {
        let request = ExtnRequest::DataGovernance(DataGovernanceRequestPayload::Call(
            DataGovernanceRequestType::SetOptOut(value.value),
        ));
        if let Ok(response) = self.extn_helper.handle(request).await {
            match response {
                ExtnResponse::DataGovernance(d) => match d {
                    DataGovernanceResponsePayload::String(s) => return Ok(s),
                    _ => {}
                },
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
        let _ = m.merge(
            XvpPrivacyRpcImpl {
                extn_helper: helper,
            }
            .into_rpc(),
        );
        m
    }
}

impl RippleExtnMeta for XvpPrivacyExtn {
    fn require(&self) -> Version {
        Version::new(1, 1, 0)
    }

    fn cap(&self) -> RippleCap {
        RippleCap::get_extn(
            ripple_sdk::plugin::RippleExtnClass::Jsonrpsee,
            "privacy".into(),
            Some("xvp".into()),
        )
    }
}

#[cfg(test)]
mod tests {

    mod channel_tests {
        use ripple_sdk::extn::dist::governance::{
            mock::mock_governance_channel, DataGovernanceChannel, DataGovernanceRequest,
            DataGovernanceRequestPayload, DataGovernanceResponsePayload,
        };
        use tokio::sync::oneshot;

        use crate::governance::xvp_privacy::XVPPrivacyChannel;

        #[tokio::test]
        async fn test_channel() {
            let c = Box::new(XVPPrivacyChannel::default());
            let s = mock_governance_channel(c).await;
            let (tx, tr) = oneshot::channel::<DataGovernanceResponsePayload>();
            let req = DataGovernanceRequest {
                payload: DataGovernanceRequestPayload::Call(
                    ripple_sdk::extn::dist::governance::DataGovernanceRequestType::SetOptOut(
                        "someValue".into(),
                    ),
                ),
                callback: tx,
            };
            let _ = s.send(req).await;
            if let Err(e) = tr.await {
                panic!("set failure")
            }

            let (tx, tr) = oneshot::channel::<DataGovernanceResponsePayload>();
            let req = DataGovernanceRequest {
                payload: DataGovernanceRequestPayload::Call(
                    ripple_sdk::extn::dist::governance::DataGovernanceRequestType::GetOptOut,
                ),
                callback: tx,
            };
            let _ = s.send(req).await;
            if let Ok(r) = tr.await {
                match r {
                    DataGovernanceResponsePayload::String(s) => {
                        assert_eq!(s, String::from("someValue"))
                    }
                    _ => panic!("get failure"),
                }
            }
        }
    }
}
