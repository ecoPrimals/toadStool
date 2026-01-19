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
        let mut buffer = self.metrics_buffer.write().await;
        buffer.push_back(metrics.clone());

        // Keep buffer size manageable
        if buffer.len() > self.config.batch_size * 2 {
            buffer.pop_front();
        }

        // Update aggregated metrics
        self.update_aggregated_metrics(workload_id, &metrics).await;

        // Adjust sampling if adaptive sampling is enabled
        if self.config.adaptive_sampling {
            self.adjust_sampling_interval(&metrics).await;
        }
    }

    /// Update aggregated metrics
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

        let mut current_load = self.current_load.write().await;
        *current_load = load;

        let mut sampling_interval = self.current_sampling_interval.write().await;

        if load > 0.8 {
            // High load - sample more frequently
            *sampling_interval = Duration::from_millis(
                (self.config.base_sampling_interval.as_millis() as f64
                    * self.config.high_load_multiplier) as u64,
            );
        } else if load < 0.2 {
            // Low load - sample less frequently
            *sampling_interval = Duration::from_millis(
                (self.config.base_sampling_interval.as_millis() as f64
                    * self.config.low_load_multiplier) as u64,
            );
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
