use crate::util::cloud_sync_monitor_utils::{get_sync_settings, replace_uri_variables};
use crate::{
    gateway::appsanity_gateway::AppsanityConfig,
    util::{cloud_linchpin_monitor::CloudLinchpinMonitor, cloud_periodic_sync::CloudPeriodicSync},
};
use dpab_core::{
    gateway::DpabDelegate,
    message::{DpabRequest, DpabRequestPayload, DpabResponsePayload},
    model::sync_and_monitor::SyncAndMonitorRequest,
};
use tonic::async_trait;
use tracing::debug;
pub struct SyncAndMonitorService {
    pub cloud_periodic_sync: CloudPeriodicSync,
    pub cloud_linchpin_monitor: CloudLinchpinMonitor,
    pub cloud_services: AppsanityConfig,
}

#[async_trait]
impl DpabDelegate for SyncAndMonitorService {
    async fn handle(&mut self, request: DpabRequest) {
        debug!("SyncAndMonitor service receive request: {:?}", request);
        if let DpabRequestPayload::SyncAndMonitor(sync_request) = request.payload {
            match sync_request {
                SyncAndMonitorRequest::SyncAndMonitor(module, dist_session, callback) => {
                    let sync_setting = get_sync_settings(
                        module,
                        self.cloud_services.clone(),
                        dist_session.clone(),
                        callback,
                    );
                    if !sync_setting.cloud_monitor_topic.is_empty() {
                        self.cloud_linchpin_monitor
                            .subscribe(
                                sync_setting.clone(),
                                &self.cloud_services.sync_monitor_service.linchpin_url,
                                &dist_session.device_id,
                                &dist_session.token,
                            )
                            .await;
                    } else {
                        debug!(
                            "service: {} Not configured with listen topic so not changes from linchpin",
                            sync_setting.cloud_service_url
                        );
                    }
                    // Periodically fetch from XVP
                    if sync_setting.cloud_sync_ttl > 0 {
                        self.cloud_periodic_sync.sync(sync_setting).await;
                        if let Some(cb) = request.callback {
                            cb.send(Ok(DpabResponsePayload::None)).ok();
                        }
                    } else {
                        debug!(
                            "service: {} Not configured with TTL so not starting periodic sync",
                            sync_setting.cloud_service_url
                        );
                    }
                }
                SyncAndMonitorRequest::UpdateDistributorToken(token) => {
                    tracing::debug!("asking cloud monitor to update SAT");
                    self.cloud_linchpin_monitor.update_sat(token).await;
                    if let Some(cb) = request.callback {
                        cb.send(Ok(DpabResponsePayload::None)).ok();
                    }
                }
            }
        }
    }
}
