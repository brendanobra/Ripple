pub mod gateway {
    pub mod appsanity_gateway;
}

#[cfg(feature = "gateway")]
pub mod gw {
    use tokio::sync::mpsc::{Receiver, Sender};
    use tracing::instrument;

    use dpab_core::{
        gateway::{Gateway, GatewayContext},
        message::{DistributorSession, DpabRequest},
    };

    use crate::gateway::appsanity_gateway::AppsanityGateway;
    use serde_json::Value;

    #[instrument(skip(rx, config))]
    pub async fn run_dpab_appsanity(
        session: Option<DistributorSession>,
        tx: Sender<DpabRequest>,
        rx: Receiver<DpabRequest>,
        config: Option<Value>,
    ) {
        let gateway_context = GatewayContext {
            session,
            sender: tx,
            receiver: rx,
            config: config,
        };
        Box::new(AppsanityGateway {}).start(gateway_context).await;
        #[cfg(any(feature = "local_metrics_logging"))]
        crate::service::appsanity_metrics::config_testing_logging();
    }
}

pub mod service {
    pub mod appsanity_account_link;
    pub mod appsanity_advertising;
    pub mod appsanity_auth;
    pub mod appsanity_discovery;
    pub mod appsanity_metrics;
    pub mod appsanity_permission;
    pub mod appsanity_privacy;
    pub mod appsanity_resolver;
    pub mod distp_secure_storage;
    pub mod thor_permission;
    pub mod xvp_sync_and_monitor;
    pub mod catalog {
        pub mod appsanity_catalog;
        pub mod catalog_persistence;
    }
}

pub mod util {
    pub mod channel_util;
    pub mod cloud_linchpin_monitor;
    pub mod cloud_periodic_sync;
    pub mod cloud_sync_monitor_utils;
    pub mod linchpin_proxy;
    pub mod service_util;
    pub mod sync_settings;
}
pub mod manager {
    pub mod metrics_manager;
}
pub mod client {
    pub mod xvp_playback;
    pub mod xvp_session;
    pub mod xvp_videoservice;
}

pub mod sync_and_monitor {
    pub mod privacy_sync_monitor;
    pub mod user_grants_sync_monitor;
}

#[cfg(feature = "extn")]
pub mod extn {
    pub mod appsanity_permission_processor;
    pub mod eos_ffi;
}

pub mod permission_service {
    tonic::include_proto!("ottx.permission");
}
pub mod ad_platform {
    tonic::include_proto!("ottx.adplatform");
}
pub mod session_service {
    tonic::include_proto!("ottx.resapi");
}
pub mod secure_storage_service {
    tonic::include_proto!("distp.gateway.secure_storage.v1");
}

pub mod catalog_service {
    tonic::include_proto!("distp.gateway.catalog.v1");
}

extern crate tokio;
