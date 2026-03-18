// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource monitoring and metrics collection
//!
//! This module provides optimized resource monitoring with adaptive sampling
//! and metrics aggregation.

use super::types::{AggregatedMetrics, OptimizedMonitoringConfig};
use crate::resources::RuntimeMetrics;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Optimized resource monitor
pub struct OptimizedResourceMonitor {
    /// Configuration
    config: OptimizedMonitoringConfig,
    /// Metrics buffer
    metrics_buffer: Arc<RwLock<VecDeque<RuntimeMetrics>>>,
    /// Aggregated metrics
    aggregated_metrics: Arc<RwLock<HashMap<String, AggregatedMetrics>>>,
    /// Current load level
    current_load: Arc<RwLock<f64>>,
    /// Sampling interval
    current_sampling_interval: Arc<RwLock<Duration>>,
}

impl OptimizedResourceMonitor {
    /// Create new optimized resource monitor
    #[must_use]
    pub fn new(config: OptimizedMonitoringConfig) -> Self {
        Self {
            config: config.clone(),
            metrics_buffer: Arc::new(RwLock::new(VecDeque::new())),
            aggregated_metrics: Arc::new(RwLock::new(HashMap::new())),
            current_load: Arc::new(RwLock::new(0.0)),
            current_sampling_interval: Arc::new(RwLock::new(config.base_sampling_interval)),
        }
    }

    /// Add metrics sample
    pub async fn add_sample(&self, workload_id: &str, metrics: RuntimeMetrics) {
        {
            let mut buffer = self.metrics_buffer.write().await;
            buffer.push_back(metrics.clone());

            // Keep buffer size manageable
            if buffer.len() > self.config.batch_size * 2 {
                buffer.pop_front();
            }
        }

        // Update aggregated metrics
        self.update_aggregated_metrics(workload_id, &metrics).await;

        // Adjust sampling if adaptive sampling is enabled
        if self.config.adaptive_sampling {
            self.adjust_sampling_interval(&metrics).await;
        }
    }

    /// Update aggregated metrics
    #[allow(clippy::significant_drop_tightening)] // agg_metrics borrows from guard for two field updates
    async fn update_aggregated_metrics(&self, workload_id: &str, metrics: &RuntimeMetrics) {
        let mut aggregated = self.aggregated_metrics.write().await;
        let agg_metrics = aggregated
            .entry(workload_id.to_string())
            .or_insert_with(|| AggregatedMetrics {
                cpu_usage: 0.0,
                memory_usage: 0,
                active_connections: 0,
                request_rate: 0.0,
                avg_response_time: 0.0,
            });

        // Update metrics (simplified - in real impl would maintain running stats)
        agg_metrics.cpu_usage = metrics.cpu.usage_percent;
        agg_metrics.memory_usage = metrics.memory.used_bytes;
    }

    /// Adjust sampling interval based on system load
    async fn adjust_sampling_interval(&self, metrics: &RuntimeMetrics) {
        let load = (metrics.cpu.usage_percent + metrics.memory.usage_percent) / 200.0;

        *self.current_load.write().await = load;

        let mut sampling_interval = self.current_sampling_interval.write().await;

        if load > 0.8 {
            // High load - sample more frequently
            let base_ms = self.config.base_sampling_interval.as_millis();
            let mult = self.config.high_load_multiplier;
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let ms = (base_ms as f64 * mult) as u64;
            *sampling_interval = Duration::from_millis(ms);
        } else if load < 0.2 {
            // Low load - sample less frequently
            let base_ms = self.config.base_sampling_interval.as_millis();
            let mult = self.config.low_load_multiplier;
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let ms = (base_ms as f64 * mult) as u64;
            *sampling_interval = Duration::from_millis(ms);
        } else {
            // Normal load - use base interval
            *sampling_interval = self.config.base_sampling_interval;
        }
    }

    /// Get aggregated metrics
    pub async fn get_aggregated_metrics(&self, workload_id: &str) -> Option<AggregatedMetrics> {
        let aggregated = self.aggregated_metrics.read().await;
        aggregated.get(workload_id).cloned()
    }

    /// Get current sampling interval
    pub async fn get_sampling_interval(&self) -> Duration {
        *self.current_sampling_interval.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{CpuMetrics, MemoryMetrics, RuntimeMetrics};
    use std::time::Duration;

    fn create_test_metrics(cpu_percent: f64, memory_percent: f64) -> RuntimeMetrics {
        RuntimeMetrics {
            cpu: CpuMetrics {
                usage_percent: cpu_percent,
                cores_used: cpu_percent / 100.0 * 4.0,
                cpu_time_seconds: 1.0,
            },
            memory: MemoryMetrics {
                usage_percent: memory_percent,
                #[allow(clippy::cast_possible_truncation)]
                used_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
                #[allow(clippy::cast_possible_truncation)]
                peak_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_optimized_resource_monitor_construction() {
        let config = OptimizedMonitoringConfig::default();
        let monitor = OptimizedResourceMonitor::new(config.clone());

        let interval = monitor.get_sampling_interval().await;
        assert_eq!(interval, config.base_sampling_interval);
    }

    #[tokio::test]
    async fn test_add_sample_and_aggregated_metrics() {
        let config = OptimizedMonitoringConfig::default();
        let monitor = OptimizedResourceMonitor::new(config);

        let metrics = create_test_metrics(50.0, 60.0);
        monitor.add_sample("workload-1", metrics.clone()).await;

        let aggregated = monitor.get_aggregated_metrics("workload-1").await;
        assert!(aggregated.is_some());
        let agg = aggregated.unwrap();
        assert!((agg.cpu_usage - 50.0).abs() < 1e-10);
        assert!(agg.memory_usage > 0);
    }

    #[tokio::test]
    async fn test_add_sample_multiple_workloads() {
        let config = OptimizedMonitoringConfig::default();
        let monitor = OptimizedResourceMonitor::new(config);

        monitor
            .add_sample("w1", create_test_metrics(10.0, 20.0))
            .await;
        monitor
            .add_sample("w2", create_test_metrics(80.0, 90.0))
            .await;

        let a1 = monitor.get_aggregated_metrics("w1").await.unwrap();
        let a2 = monitor.get_aggregated_metrics("w2").await.unwrap();

        assert!((a1.cpu_usage - 10.0).abs() < 1e-10);
        assert!((a2.cpu_usage - 80.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn test_adjust_sampling_interval_high_load() {
        let config = OptimizedMonitoringConfig {
            adaptive_sampling: true,
            base_sampling_interval: Duration::from_millis(100),
            high_load_multiplier: 0.5,
            ..Default::default()
        };
        let monitor = OptimizedResourceMonitor::new(config);

        let high_load = create_test_metrics(90.0, 90.0);
        monitor.add_sample("high", high_load).await;

        let interval = monitor.get_sampling_interval().await;
        assert_eq!(interval, Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_adjust_sampling_interval_low_load() {
        let config = OptimizedMonitoringConfig {
            adaptive_sampling: true,
            base_sampling_interval: Duration::from_millis(100),
            low_load_multiplier: 2.0,
            ..Default::default()
        };
        let monitor = OptimizedResourceMonitor::new(config);

        let low_load = create_test_metrics(10.0, 10.0);
        monitor.add_sample("low", low_load).await;

        let interval = monitor.get_sampling_interval().await;
        assert_eq!(interval, Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_adjust_sampling_interval_normal_load() {
        let config = OptimizedMonitoringConfig {
            adaptive_sampling: true,
            base_sampling_interval: Duration::from_millis(100),
            ..Default::default()
        };
        let monitor = OptimizedResourceMonitor::new(config);

        let normal_load = create_test_metrics(50.0, 50.0);
        monitor.add_sample("normal", normal_load).await;

        let interval = monitor.get_sampling_interval().await;
        assert_eq!(interval, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_get_aggregated_metrics_missing_workload() {
        let config = OptimizedMonitoringConfig::default();
        let monitor = OptimizedResourceMonitor::new(config);

        let result = monitor.get_aggregated_metrics("nonexistent").await;
        assert!(result.is_none());
    }
}
