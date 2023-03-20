use ripple_sdk::framework::bootstrap::Bootstrap;

use crate::state::bootstrap_state::BootstrapState;

use super::{
    extn::{
        check_launcher_step::CheckLauncherStep, load_extn_metadata_step::LoadExtensionMetadataStep,
        load_extn_step::LoadExtensionsStep, start_extn_channel_step::StartExtnChannelsStep,
    },
    setup_extn_client_step::SetupExtnClientStep,
    start_app_manager_step::StartAppManagerStep,
    start_fbgateway_step::FireboltGatewayStep,
    start_ws_step::StartWsStep,
};
/// Starts up Ripple uses `PlatformState` to manage State
/// # Arguments
/// * `platform_state` - PlatformState
///
/// # Panics
///
/// Bootstrap panics are fatal in nature and it could happen due to bad configuration or device state. Logs should provide more information on which step the failure had occurred.
///
/// # Steps
///
/// 1. [SetupExtnClientStep] - Initializes the extn client to start the Inter process communication backbone
/// 2. [LoadExtensionMetadataStep] - Loads the Extn metadata from the So files
/// 3. [LoadExtensionsStep] - Loads the Extensions in to [crate::state::extn_state::ExtnState]
/// 4. [StartExtnChannelsStep] - Starts the Device channel extension
/// 5. [StartAppManagerStep] - Starts the App Manager and other supporting services
/// 6. [CheckLauncherStep] - Checks the presence of launcher extension and starts default app
/// 7. [StartWsStep] - Starts the Websocket to accept external and internal connections
/// 8. [FireboltGatewayStep] - Starts the firebolt gateway and blocks the thread to keep it alive till interruption.
///
pub async fn boot(state: BootstrapState) {
    let bootstrap = &Bootstrap::new(state);
    bootstrap
        .step(SetupExtnClientStep)
        .await
        .expect("Extn Client setup failure")
        .step(LoadExtensionMetadataStep)
        .await
        .expect("Extn Metadata load failure")
        .step(LoadExtensionsStep)
        .await
        .expect("Extn load failure")
        .step(StartExtnChannelsStep)
        .await
        .expect("Start Device channel failure")
        .step(StartAppManagerStep)
        .await
        .expect("App Manager start")
        .step(CheckLauncherStep)
        .await
        .expect("if Launcher exists start")
        .step(StartWsStep)
        .await
        .expect("Websocket startup failure")
        .step(FireboltGatewayStep)
        .await
        .expect("Firebolt Gateway failure");
    // -- capability Manager
    // -- App event manager
    // -- User grant manager
    // -- permissions manager
    // -- app manager
    // Start Launcher
    // Start Dpab
    // Start Launcher
}
