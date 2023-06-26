use rust_linchpin_client::{
    linchpin_client::{self, LinchpinClient},
    model::client_messages::{ClientConfig, ClientEvent},
};
use tokio::sync::mpsc::{self, Sender};

use super::cloud_sync_monitor_utils::LinchpinProxyCommand;

use tracing::{debug, error};

#[derive(Debug)]
pub struct LinchpinProxy {}

impl LinchpinProxy {
    fn get_linchpin_url(base_url: &str, dev_id: &str) -> String {
        if base_url.ends_with("/") {
            format!("{}listen?client=ripple&deviceId={}", base_url, dev_id)
        } else {
            format!("{}/listen?client=ripple&deviceId={}", base_url, dev_id)
        }
    }

    pub fn start(client_event_tx: Sender<ClientEvent>) -> Sender<LinchpinProxyCommand> {
        let (tx, mut rx) = mpsc::channel::<LinchpinProxyCommand>(32);
        let mut linchpin: Option<LinchpinClient> = None;
        tokio::spawn(async move {
            debug!("Starting linchpin proxy");
            while let Some(message) = rx.recv().await {
                match message {
                    LinchpinProxyCommand::Subscribe(topic) => {
                        if let Some(client) = linchpin.as_mut() {
                            let result = client.subscribe(&topic).await;
                            if let Err(e) = result {
                                error!("unable to subscribe to topic: {} error: {:?}", topic, e);
                            }
                        }
                    }
                    LinchpinProxyCommand::Unsubscribe(topic) => {
                        if let Some(client) = linchpin.as_mut() {
                            let result = client.unsubscribe(&topic).await;
                            if let Err(e) = result {
                                error!("unable to unsubscribe to topic: {} error:{:?}", topic, e);
                            }
                        }
                    }
                    LinchpinProxyCommand::Connect(linchpin_url, device_id, sat) => {
                        debug!(
                            "Received Linchpin connect for dev_id: {} with sat: {}",
                            device_id, sat
                        );
                        debug!(
                            "linchpin url: {:?}",
                            LinchpinProxy::get_linchpin_url(&linchpin_url, &device_id)
                        );
                        let client_config = ClientConfig {
                            url: LinchpinProxy::get_linchpin_url(&linchpin_url, &device_id),
                            sat,
                            initial_reconnect_delay_ms: None,
                            max_reconnect_delay_ms: None,
                            heartbeat_timeout_secs: None,
                        };
                        let _result = linchpin.insert(
                            linchpin_client::start(client_config, client_event_tx.clone()).await,
                        );
                    }
                    LinchpinProxyCommand::UpdateDistributorToken(token) => {
                        if let Some(client) = linchpin.as_mut() {
                            debug!("Updating linchpin client with new SAT");
                            let res = client.set_sat(token).await;
                            if let Err(e) = res {
                                tracing::error!("Unable to update SAT to linchpin client: {:?}", e);
                            }
                        } else {
                            error!("unable to update SAT to linchpin client as it is not ready");
                        }
                    }
                }
            }
        });
        tx
    }
}
