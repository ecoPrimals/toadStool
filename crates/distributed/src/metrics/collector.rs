// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Universal metrics collector
pub struct UniversalMetricsCollector {
    /// Local metrics
    _local_metrics: Arc<RwLock<LocalMetrics>>,
    /// Network metrics
    _network_metrics: Arc<RwLock<NetworkMetrics>>,
    /// Ecosystem metrics
    _ecosystem_metrics: Arc<RwLock<EcosystemMetrics>>,
    /// Recursive hosting metrics
    _recursive_metrics: Arc<RwLock<RecursiveHostingMetrics>>,
}

/// Local metrics
#[derive(Debug, Clone, Default)]
pub struct LocalMetrics {
    pub active_jobs: u64,
    pub total_processed: u64,
    pub success_rate: f64,
    pub average_execution_time: Duration,
}

/// Network metrics
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub active_network_jobs: u64,
    pub network_utilization: f64,
    pub average_latency: Duration,
}

/// Ecosystem metrics
#[derive(Debug, Clone, Default)]
pub struct EcosystemMetrics {
    pub active_ecosystem_jobs: u64,
    pub ecosystem_service_calls: HashMap<String, u64>,
    pub ecosystem_success_rates: HashMap<String, f64>,
}

/// Recursive hosting metrics
#[derive(Debug, Clone)]
pub struct RecursiveHostingMetrics {
    pub active_child_instances: u64,
    pub total_child_instances_created: u64,
    pub child_instance_success_rate: f64,
}

impl UniversalMetricsCollector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _local_metrics: Arc::new(RwLock::new(LocalMetrics::default())),
            _network_metrics: Arc::new(RwLock::new(NetworkMetrics::default())),
            _ecosystem_metrics: Arc::new(RwLock::new(EcosystemMetrics::default())),
            _recursive_metrics: Arc::new(RwLock::new(RecursiveHostingMetrics::default())),
        }
    }
}

impl Default for UniversalMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            active_network_jobs: 0,
            network_utilization: 0.0,
            average_latency: Duration::from_millis(0),
        }
    }
}

impl Default for RecursiveHostingMetrics {
    fn default() -> Self {
        Self {
            active_child_instances: 0,
            total_child_instances_created: 0,
            child_instance_success_rate: 0.0,
        }
    }
}
