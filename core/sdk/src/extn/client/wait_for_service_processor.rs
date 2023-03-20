use crate::{
    api::status_update::ExtnStatus,
    async_trait::async_trait,
    extn::{
        client::extn_processor::{
            DefaultExtnStreamer, ExtnEventProcessor, ExtnStreamProcessor, ExtnStreamer,
        },
        extn_client_message::ExtnMessage,
        extn_id::ExtnId,
    },
    log::error,
    tokio::sync::{mpsc::Receiver as MReceiver, mpsc::Sender as MSender},
};

#[derive(Debug, Clone)]
pub struct WaitForState {
    capability: ExtnId,
    sender: MSender<ExtnStatus>,
}

#[derive(Debug)]
pub struct WaitForStatusReadyEventProcessor {
    state: WaitForState,
    streamer: DefaultExtnStreamer,
}

/// Event processor used for cases where a certain Extension Capability is required to be ready.
/// Bootstrap uses the [WaitForStatusReadyEventProcessor] to await during Device Connnection before starting the gateway.
impl WaitForStatusReadyEventProcessor {
    pub fn new(
        capability: ExtnId,
        sender: MSender<ExtnStatus>,
    ) -> WaitForStatusReadyEventProcessor {
        WaitForStatusReadyEventProcessor {
            state: WaitForState { capability, sender },
            streamer: DefaultExtnStreamer::new(),
        }
    }
}

impl ExtnStreamProcessor for WaitForStatusReadyEventProcessor {
    type VALUE = ExtnStatus;
    type STATE = WaitForState;

    fn get_state(&self) -> Self::STATE {
        self.state.clone()
    }

    fn sender(&self) -> MSender<ExtnMessage> {
        self.streamer.sender()
    }

    fn receiver(&mut self) -> MReceiver<ExtnMessage> {
        self.streamer.receiver()
    }
}

#[async_trait]
impl ExtnEventProcessor for WaitForStatusReadyEventProcessor {
    async fn process_event(
        state: Self::STATE,
        msg: ExtnMessage,
        extracted_message: Self::VALUE,
    ) -> Option<bool> {
        if msg.requestor.to_string().eq(&state.capability.to_string()) {
            match extracted_message {
                ExtnStatus::Ready => {
                    if let Err(_) = state.sender.send(ExtnStatus::Ready).await {
                        error!("Failure to wait status message")
                    }
                    return Some(true);
                }
                _ => {}
            }
        }
        None
    }
}
