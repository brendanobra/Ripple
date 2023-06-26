use rust_linchpin_client::model::client_messages::{ClientEvent, ClientState};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tracing::{debug, error};

use super::cloud_sync_monitor_utils::{LinchpinProxyCommand, StateRequest};
use crate::util::sync_settings::SyncSettings;
use crate::util::{cloud_sync_monitor_utils::LinchpinPayload, linchpin_proxy::LinchpinProxy};

#[derive(Debug, Clone)]
pub struct CloudLinchpinMonitor {
    command_tx: Sender<LinchpinProxyCommand>,
    state_tx: Sender<StateRequest>,
}

impl CloudLinchpinMonitor {
    pub fn start(state_tx: Sender<StateRequest>) -> Self {
        let (client_event_tx, mut client_event_rx) = mpsc::channel::<ClientEvent>(32);
        let command_tx = LinchpinProxy::start(client_event_tx);

        let state_tx_c = state_tx.clone();
        let cmd_tx_c = command_tx.clone();
        let _ = tokio::spawn(async move {
            while let Some(client_event) = client_event_rx.recv().await {
                let state_tx_clone = state_tx_c.clone();
                let cmd_tx_clone = cmd_tx_c.clone();
                Self::process_linchpin_notification(state_tx_clone, cmd_tx_clone, &client_event)
                    .await;
            }
        });
        CloudLinchpinMonitor {
            command_tx,
            state_tx,
        }
    }

    pub async fn subscribe(
        &self,
        settings: SyncSettings,
        linchpin_url: &str,
        dev_id: &str,
        sat: &str,
    ) {
        let listen_topic = settings.cloud_monitor_topic.to_owned();
        debug!("linchpin subscribe topic: {listen_topic}");
        let (callback_listen_list_tx, callback_listen_list_rx) = oneshot::channel();
        let (callback_lp_connected_tx, callback_lp_connected_rx) = oneshot::channel::<bool>();
        let _ = self
            .state_tx
            .send(StateRequest::SetDistributorToken(sat.to_owned()))
            .await;
        let _ = self
            .state_tx
            .send(StateRequest::GetLinchpinConnectionStatus(
                callback_lp_connected_tx,
            ))
            .await;
        if let Ok(connection_status) = callback_lp_connected_rx.await {
            debug!("linchpin conneced status: {connection_status}");
            if connection_status {
                let _ = self
                    .state_tx
                    .send(StateRequest::GetListeningTopics(callback_listen_list_tx))
                    .await;
                if let Ok(listen_topic_list) = callback_listen_list_rx.await {
                    if !listen_topic_list.contains(&listen_topic) {
                        let _ = self
                            .command_tx
                            .send(LinchpinProxyCommand::Subscribe(listen_topic))
                            .await;
                    }
                }
            } else {
                debug!("Adding topic to pending: {listen_topic}");
                let _ = self
                    .state_tx
                    .send(StateRequest::AddPendingTopic(listen_topic))
                    .await;
                let _ = self
                    .command_tx
                    .send(LinchpinProxyCommand::Connect(
                        linchpin_url.to_owned(),
                        dev_id.to_owned(),
                        sat.to_owned(),
                    ))
                    .await;
            }
            debug!("Adding listner :{:?}", settings);
            let _ = self
                .state_tx
                .send(StateRequest::AddListener(settings))
                .await;
        } else {
            error!("Unable to receive linchpin connection status");
        }
    }

    pub async fn update_sat(&self, sat: String) {
        debug!("sending command to proxy to update sat");
        let _ = self
            .state_tx
            .send(StateRequest::SetDistributorToken(sat.to_owned()))
            .await;
        let _ = self
            .command_tx
            .send(LinchpinProxyCommand::UpdateDistributorToken(sat))
            .await;
    }

    async fn process_linchpin_notification(
        state_tx: Sender<StateRequest>,
        cmd_tx: Sender<LinchpinProxyCommand>,
        client_event: &ClientEvent,
    ) {
        debug!("about to linchpin process event: {:?}", client_event);
        match client_event {
            ClientEvent::State(client_state) => {
                if client_state == &ClientState::Connected {
                    let _ = state_tx
                        .send(StateRequest::SetLinchpinConnectionStatus(true))
                        .await;
                    let (callback_pending_topic_tx, mut callback_pending_topic_rx) =
                        oneshot::channel();
                    let _ = state_tx
                        .send(StateRequest::GetAllPendingTopics(callback_pending_topic_tx))
                        .await;
                    if let Ok(pending_list) = callback_pending_topic_rx.await {
                        for pending_topic in pending_list {
                            debug!("Sending subscribe event for topic: {:?}", pending_topic);
                            let _ = cmd_tx
                                .clone()
                                .send(LinchpinProxyCommand::Subscribe(pending_topic.to_owned()))
                                .await;
                        }
                        let _ = state_tx.send(StateRequest::ClearPendingTopics).await;
                    }
                }
            }
            ClientEvent::Notify(message) => {
                let topic = message.topic.as_str();
                let res_linchpin_payload =
                    serde_json::from_str::<LinchpinPayload>(message.payload.as_str());
                debug!("Received Linchpin Payload: {:?}", res_linchpin_payload);
                if let Err(_) = res_linchpin_payload {
                    error!(
                        "Unable to parse received linchpin payload: {:?}",
                        message.payload
                    );
                    return;
                }
                let linchpin_payload = res_linchpin_payload.unwrap();
                let updated_settings = linchpin_payload
                    .event_payload
                    .settings
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect();
                let (tx, rx) = oneshot::channel::<Vec<SyncSettings>>();
                let _ = state_tx
                    .send(StateRequest::GetListenersForProperties(
                        topic.to_owned(),
                        updated_settings,
                        tx,
                    ))
                    .await;
                if let Ok(listeners) = rx.await {
                    for listener in listeners {
                        let response = listener.get_values_from_cloud(state_tx.clone()).await;
                        if let Ok(resp) = response {
                            let _ = listener.callback.send(resp).await;
                        }
                    }
                }
            }
        }
    }
}
