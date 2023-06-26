use async_trait::async_trait;
use dpab_core::message::DistributorSession;
use dpab_core::model::metrics::AppBehavioralMetric;
use dpab_core::model::metrics::BehavioralMetricsService;
use tracing::info;
struct LoggingBehavioralMetricsManager {}
impl LoggingBehavioralMetricsManager {}
#[async_trait]
impl BehavioralMetricsService for LoggingBehavioralMetricsManager {
    async fn send_metric(
        &mut self,
        metrics: AppBehavioralMetric,
        _session: DistributorSession,
    ) -> () {
        info!("{:?}", metrics);
    }
}
