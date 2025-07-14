use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Network metrics collector
pub struct NetworkMetricsCollector {
    metrics: Arc<RwLock<NetworkMetricsData>>,
}

/// Network metrics data
#[derive(Debug, Clone)]
pub struct NetworkMetricsData {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: Duration,
}

impl NetworkMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(NetworkMetricsData::default())),
        }
    }
}

impl Default for NetworkMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NetworkMetricsData {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time: Duration::from_millis(0),
        }
    }
}
