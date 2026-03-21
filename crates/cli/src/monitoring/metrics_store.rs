// SPDX-License-Identifier: AGPL-3.0-only
//! Metrics storage and time series data

use std::collections::HashMap;
use tokio::time::Duration;

use crate::monitoring::types::{DataPoint, MetricBatch, MetricStats, MetricValue, TimeSeries};

/// Metrics storage and time series data
pub struct MetricsStore {
    pub(crate) series: HashMap<String, TimeSeries>,
    pub(crate) stats: HashMap<String, MetricStats>,
    pub(crate) retention_period: Duration,
}

impl MetricsStore {
    /// Create a new metrics store with the given retention period
    pub fn new(retention_period: Duration) -> Self {
        Self {
            series: HashMap::new(),
            stats: HashMap::new(),
            retention_period,
        }
    }

    /// Store a batch of metrics and update stats
    pub async fn store_batch(&mut self, batch: MetricBatch) {
        for metric in batch.metrics {
            let series = self
                .series
                .entry(metric.name.clone())
                .or_insert_with(|| TimeSeries {
                    name: metric.name.clone(),
                    data_points: Vec::new(),
                    labels: metric.labels.clone(),
                });

            if let MetricValue::Gauge(value) = metric.value {
                series.data_points.push(DataPoint {
                    timestamp: metric.timestamp,
                    value,
                });

                self.update_stats(&metric.name, value);
            }
        }

        self.cleanup_old_data().await;
    }

    /// Update min/max/avg stats for a metric
    pub fn update_stats(&mut self, metric_name: &str, value: f64) {
        let stats = self
            .stats
            .entry(metric_name.to_string())
            .or_insert_with(|| MetricStats {
                count: 0,
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
                avg: 0.0,
                percentiles: HashMap::new(),
            });

        stats.count += 1;
        stats.min = stats.min.min(value);
        stats.max = stats.max.max(value);
        stats.avg = stats.avg.mul_add((stats.count - 1) as f64, value) / stats.count as f64;
    }

    #[allow(clippy::unused_async)]
    async fn cleanup_old_data(&mut self) {
        let now = std::time::SystemTime::now();
        if let Some(cutoff_time) = now.checked_sub(self.retention_period) {
            for series in self.series.values_mut() {
                series
                    .data_points
                    .retain(|point| point.timestamp > cutoff_time);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::types::Metric;

    #[test]
    fn test_metrics_store_new() {
        let store = MetricsStore::new(Duration::from_secs(3600));
        assert!(store.series.is_empty());
        assert!(store.stats.is_empty());
        assert_eq!(store.retention_period, Duration::from_secs(3600));
    }

    #[test]
    fn test_update_stats_first_value() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("cpu", 42.0);
        let stats = store.stats.get("cpu").expect("stats");
        assert_eq!(stats.count, 1);
        assert!((stats.min - 42.0).abs() < f64::EPSILON);
        assert!((stats.max - 42.0).abs() < f64::EPSILON);
        assert!((stats.avg - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_stats_multiple_values() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("mem", 10.0);
        store.update_stats("mem", 20.0);
        store.update_stats("mem", 30.0);
        let stats = store.stats.get("mem").expect("stats");
        assert_eq!(stats.count, 3);
        assert!((stats.min - 10.0).abs() < f64::EPSILON);
        assert!((stats.max - 30.0).abs() < f64::EPSILON);
        assert!((stats.avg - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_stats_independent_metrics() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("a", 5.0);
        store.update_stats("b", 100.0);
        assert_eq!(store.stats.len(), 2);
        assert!((store.stats["a"].avg - 5.0).abs() < f64::EPSILON);
        assert!((store.stats["b"].avg - 100.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_cleanup_old_data_removes_expired() {
        let mut store = MetricsStore::new(Duration::from_secs(60));
        let old = std::time::SystemTime::now()
            .checked_sub(Duration::from_secs(120))
            .expect("sub");
        let recent = std::time::SystemTime::now();
        store.series.insert(
            "test".to_string(),
            TimeSeries {
                name: "test".to_string(),
                data_points: vec![
                    DataPoint {
                        timestamp: old,
                        value: 1.0,
                    },
                    DataPoint {
                        timestamp: recent,
                        value: 2.0,
                    },
                ],
                labels: HashMap::new(),
            },
        );
        store.cleanup_old_data().await;
        assert_eq!(store.series["test"].data_points.len(), 1);
        assert!((store.series["test"].data_points[0].value - 2.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_store_batch_gauge_metric() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        let batch = MetricBatch {
            timestamp: std::time::SystemTime::now(),
            source: "test".to_string(),
            metrics: vec![Metric {
                name: "cpu".to_string(),
                value: MetricValue::Gauge(75.0),
                labels: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            }],
        };
        store.store_batch(batch).await;
        assert_eq!(store.series.len(), 1);
        assert_eq!(store.series["cpu"].data_points.len(), 1);
        assert!((store.stats["cpu"].avg - 75.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_store_batch_counter_ignored() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        let batch = MetricBatch {
            timestamp: std::time::SystemTime::now(),
            source: "test".to_string(),
            metrics: vec![Metric {
                name: "requests".to_string(),
                value: MetricValue::Counter(100),
                labels: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            }],
        };
        store.store_batch(batch).await;
        assert!(!store.stats.contains_key("requests"));
    }
}
