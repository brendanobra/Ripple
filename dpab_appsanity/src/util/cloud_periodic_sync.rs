use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;

use super::cloud_sync_monitor_utils::StateRequest;
use crate::util::sync_settings::SyncSettings;
use tokio::time::{interval, Duration};

use tracing::debug;
#[derive(Debug, Clone)]
pub struct CloudPeriodicSync {
    monitor_tx: Sender<SyncSettings>,
}

impl CloudPeriodicSync {
    pub fn start(state_tx: Sender<StateRequest>) -> Self {
        let (tx, mut rx) = mpsc::channel::<SyncSettings>(32);
        let state_tx_c = state_tx.clone();
        tokio::spawn(async move {
            while let Some(sync_settings) = rx.recv().await {
                let stc = state_tx_c.clone();
                let topic = sync_settings.cloud_monitor_topic.to_owned();
                debug!("sync settings: {:?}", sync_settings);
                tokio::spawn(async move {
                    let mut interval =
                        interval(Duration::from_secs(sync_settings.cloud_sync_ttl as u64));
                    loop {
                        debug!("Waiting for time to expire");
                        interval.tick().await;
                        debug!("time expired");
                        let value = sync_settings.get_values_from_cloud(stc.clone()).await;
                        if let Ok(val) = value {
                            let (tx, rx) = oneshot::channel();
                            let _ = stc
                                .send(StateRequest::GetListenersForModule(
                                    topic.clone(),
                                    sync_settings.module,
                                    tx,
                                ))
                                .await;
                            if let Ok(listeners) = rx.await {
                                for listener in listeners {
                                    let _res = listener.callback.send(val.clone()).await;
                                }
                            }
                        } else {
                            tracing::error!(
                                "Unable to fetch values from cloud url: {:?} error: {:?}",
                                sync_settings.cloud_service_url,
                                value.err().unwrap()
                            );
                        }
                    }
                });
            }
        });
        CloudPeriodicSync { monitor_tx: tx }
    }

    pub async fn sync(&self, sync_settings: SyncSettings) {
        let _res = self.monitor_tx.send(sync_settings).await;
    }
}
