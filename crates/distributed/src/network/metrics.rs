// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Network metrics collector for distributed execution
pub struct NetworkMetricsCollector {
    #[allow(dead_code, reason = "used in tests")]
    metrics: Arc<RwLock<NetworkMetricsData>>,
}

/// Aggregated request/response metrics for network distribution.
#[derive(Debug, Clone)]
pub struct NetworkMetricsData {
    /// Total requests sent.
    pub total_requests: u64,
    /// Requests that completed successfully.
    pub successful_requests: u64,
    /// Requests that failed (timeout, error, etc.).
    pub failed_requests: u64,
    /// Mean response time across all requests.
    pub average_response_time: Duration,
}

impl NetworkMetricsCollector {
    /// Creates a new network metrics collector.
    #[must_use]
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

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "test values are exact literals")]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_data_default() {
        let data = NetworkMetricsData::default();

        assert_eq!(data.total_requests, 0);
        assert_eq!(data.successful_requests, 0);
        assert_eq!(data.failed_requests, 0);
        assert_eq!(data.average_response_time, Duration::from_millis(0));
    }

    #[test]
    fn test_metrics_data_clone() {
        let data = NetworkMetricsData {
            total_requests: 100,
            successful_requests: 95,
            failed_requests: 5,
            average_response_time: Duration::from_millis(250),
        };

        let cloned = data.clone();
        assert_eq!(cloned.total_requests, 100);
        assert_eq!(cloned.successful_requests, 95);
        assert_eq!(cloned.failed_requests, 5);
        assert_eq!(cloned.average_response_time, Duration::from_millis(250));
    }

    #[test]
    fn test_metrics_data_debug() {
        let data = NetworkMetricsData::default();
        let debug_str = format!("{:?}", data);

        assert!(debug_str.contains("NetworkMetricsData"));
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = NetworkMetricsCollector::new();
        // Verify collector was created successfully
        assert!(collector.metrics.try_read().is_ok());
    }

    #[test]
    fn test_metrics_collector_default() {
        let collector = NetworkMetricsCollector::default();
        assert!(collector.metrics.try_read().is_ok());
    }

    #[test]
    fn test_metrics_data_with_values() {
        let data = NetworkMetricsData {
            total_requests: 1000,
            successful_requests: 950,
            failed_requests: 50,
            average_response_time: Duration::from_millis(150),
        };

        assert_eq!(data.total_requests, 1000);
        assert_eq!(data.successful_requests, 950);
        assert_eq!(data.failed_requests, 50);
        assert_eq!(
            data.successful_requests + data.failed_requests,
            data.total_requests
        );
    }

    #[test]
    fn test_metrics_data_success_rate() {
        let data = NetworkMetricsData {
            total_requests: 200,
            successful_requests: 180,
            failed_requests: 20,
            average_response_time: Duration::from_millis(100),
        };

        let success_rate = (data.successful_requests as f64 / data.total_requests as f64) * 100.0;
        assert_eq!(success_rate, 90.0);
    }

    #[test]
    fn test_metrics_data_response_time_ranges() {
        let fast = NetworkMetricsData {
            total_requests: 10,
            successful_requests: 10,
            failed_requests: 0,
            average_response_time: Duration::from_millis(50),
        };

        let slow = NetworkMetricsData {
            total_requests: 10,
            successful_requests: 10,
            failed_requests: 0,
            average_response_time: Duration::from_millis(500),
        };

        assert!(fast.average_response_time < slow.average_response_time);
    }
}
