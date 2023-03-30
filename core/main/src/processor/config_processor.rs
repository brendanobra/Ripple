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

use ripple_sdk::{
    api::config::{Config, ConfigResponse, LauncherConfig},
    async_trait::async_trait,
    extn::{
        client::extn_processor::{
            DefaultExtnStreamer, ExtnRequestProcessor, ExtnStreamProcessor, ExtnStreamer,
        },
        extn_client_message::{ExtnMessage, ExtnPayload, ExtnPayloadProvider, ExtnResponse},
    },
    tokio::sync::mpsc::{Receiver as MReceiver, Sender as MSender},
};

use crate::state::platform_state::PlatformState;

/// Supports processing of [Config] request from extensions and also
/// internal services.
#[derive(Debug)]
pub struct ConfigRequestProcessor {
    state: PlatformState,
    streamer: DefaultExtnStreamer,
}

impl ConfigRequestProcessor {
    pub fn new(state: PlatformState) -> ConfigRequestProcessor {
        ConfigRequestProcessor {
            state,
            streamer: DefaultExtnStreamer::new(),
        }
    }
}

impl ExtnStreamProcessor for ConfigRequestProcessor {
    type STATE = PlatformState;
    type VALUE = Config;
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
impl ExtnRequestProcessor for ConfigRequestProcessor {
    fn get_client(&self) -> ripple_sdk::extn::client::extn_client::ExtnClient {
        self.state.get_client().get_extn_client()
    }

    async fn process_request(
        state: Self::STATE,
        msg: ExtnMessage,
        extracted_message: Self::VALUE,
    ) -> bool {
        let device_manifest = state.get_device_manifest();

        let config_request = extracted_message;
        let response = match config_request {
            Config::PlatformParameters => {
                ExtnResponse::Value(device_manifest.configuration.platform_parameters.clone())
            }
            Config::LauncherConfig => {
                let config = LauncherConfig {
                    lifecycle_policy: device_manifest.get_lifecycle_policy(),
                    retention_policy: device_manifest.get_retention_policy(),
                    app_library_state: state.clone().app_library_state,
                };
                if let ExtnPayload::Response(r) = config.get_extn_payload() {
                    r
                } else {
                    ExtnResponse::Error(ripple_sdk::utils::error::RippleError::ProcessorError)
                }
            }
            Config::AllDefaultApps => ExtnResponse::Config(ConfigResponse::AllApps(
                state.app_library_state.get_all_apps(),
            )),
            _ => ExtnResponse::Error(ripple_sdk::utils::error::RippleError::InvalidInput),
        };
        Self::respond(state.get_client().get_extn_client(), msg, response)
            .await
            .is_ok()
    }
}