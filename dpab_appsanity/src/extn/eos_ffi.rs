use dpab_core::model::thor_permission_registry::ThorPermissionRegistry;
use ripple_sdk::{
    api::status_update::ExtnStatus,
    crossbeam::channel::Receiver as CReceiver,
    export_channel_builder, export_extn_metadata,
    extn::{
        client::{extn_client::ExtnClient, extn_sender::ExtnSender},
        extn_id::{ExtnClassId, ExtnId},
        ffi::{
            ffi_channel::{ExtnChannel, ExtnChannelBuilder},
            ffi_library::{CExtnMetadata, ExtnMetadata, ExtnSymbolMetadata},
            ffi_message::CExtnMessage,
        },
    },
    framework::ripple_contract::{ContractFulfiller, RippleContract},
    log::{debug, info},
    semver::Version,
    tokio::runtime::Runtime,
    utils::{error::RippleError, logger::init_logger},
};

use crate::{
    extn::appsanity_permission_processor::DistributorPermissionProcessor,
    service::thor_permission::ThorPermissionService,
};

const EXTN_NAME: &'static str = "eos";
const EXTN_FULL_NAME: &'static str = "distributor_eos";

fn init_library() -> CExtnMetadata {
    let _ = init_logger(EXTN_FULL_NAME.into());

    let dist_meta = ExtnSymbolMetadata::get(
        ExtnId::new_channel(ExtnClassId::Distributor, EXTN_NAME.into()),
        ContractFulfiller::new(vec![RippleContract::Permissions]),
        Version::new(1, 1, 0),
    );

    let extn_metadata = ExtnMetadata {
        name: EXTN_FULL_NAME.into(),
        symbols: vec![dist_meta],
    };
    extn_metadata.into()
}

export_extn_metadata!(CExtnMetadata, init_library);

fn start(sender: ExtnSender, receiver: CReceiver<CExtnMessage>) {
    let _ = init_logger(EXTN_FULL_NAME.into());
    info!("Starting EOS channel");
    let runtime = Runtime::new().unwrap();
    let mut client = ExtnClient::new(receiver.clone(), sender);
    runtime.block_on(async move {
        // TODO get config from main
        let tps = ThorPermissionService::new(
            String::from("thor-permission.svc-qa.thor.comcast.com"),
            ThorPermissionRegistry::new(),
        );
        let perm_proc = DistributorPermissionProcessor::new(client.clone(), Box::new(tps));
        client.add_request_processor(perm_proc);
        // Lets Main know that the distributor channel is ready
        let _ = client.event(ExtnStatus::Ready);
        client.initialize().await;
    });
}

fn build(extn_id: String) -> Result<Box<ExtnChannel>, RippleError> {
    if let Ok(id) = ExtnId::try_from(extn_id.clone()) {
        let current_id = ExtnId::new_channel(ExtnClassId::Distributor, EXTN_NAME.into());

        if id.eq(&current_id) {
            return Ok(Box::new(ExtnChannel { start: start }));
        } else {
            Err(RippleError::ExtnError)
        }
    } else {
        Err(RippleError::InvalidInput)
    }
}

fn init_extn_builder() -> ExtnChannelBuilder {
    ExtnChannelBuilder {
        build,
        service: EXTN_FULL_NAME.into(),
    }
}

export_channel_builder!(ExtnChannelBuilder, init_extn_builder);
