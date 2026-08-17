// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

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

/// Local job execution metrics (active, processed, success rate, latency).
#[derive(Debug, Clone, Default)]
pub struct LocalMetrics {
    /// Number of jobs currently in flight.
    pub active_jobs: u64,
    /// Total jobs completed (success or failure).
    pub total_processed: u64,
    /// Fraction of successful completions (0.0–1.0).
    pub success_rate: f64,
    /// Mean execution time per job.
    pub average_execution_time: Duration,
}

/// Network distribution metrics (remote jobs, utilization, latency).
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    /// Jobs currently executing on remote nodes.
    pub active_network_jobs: u64,
    /// Network bandwidth/utilization fraction (0.0–1.0).
    pub network_utilization: f64,
    /// Mean round-trip latency for remote calls.
    pub average_latency: Duration,
}

/// Ecosystem service call metrics per service.
#[derive(Debug, Clone, Default)]
pub struct EcosystemMetrics {
    /// Active jobs involving ecosystem services.
    pub active_ecosystem_jobs: u64,
    /// Call count per service name.
    pub ecosystem_service_calls: HashMap<String, u64>,
    /// Success rate per service name.
    pub ecosystem_success_rates: HashMap<String, f64>,
}

/// Recursive hosting metrics for child ToadStool instances.
#[derive(Debug, Clone)]
pub struct RecursiveHostingMetrics {
    /// Currently running child instances.
    pub active_child_instances: u64,
    /// Total child instances ever created.
    pub total_child_instances_created: u64,
    /// Fraction of child instances that started successfully.
    pub child_instance_success_rate: f64,
}

impl UniversalMetricsCollector {
    /// Creates a new metrics collector with default metric stores.
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
