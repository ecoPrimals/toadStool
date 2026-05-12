// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metrics collectors - system, process, and network data collection

use crate::Result;
use std::collections::HashMap;

use crate::monitoring::types::{Metric, MetricBatch, MetricValue};

/// Metrics collection interface
pub trait MetricsCollector: Send + Sync {
    /// Collector name (e.g. system, process, network)
    fn name(&self) -> &str;
    /// Collect metrics into a batch
    fn collect(&self) -> Result<MetricBatch>;
    /// Metric types this collector provides
    fn capabilities(&self) -> Vec<String>;
}

/// System metrics collector (CPU, memory, storage)
pub struct SystemMetricsCollector;

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMetricsCollector {
    /// Create a new system metrics collector
    pub const fn new() -> Self {
        Self
    }
}

impl MetricsCollector for SystemMetricsCollector {
    fn name(&self) -> &'static str {
        "system"
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    fn collect(&self) -> Result<MetricBatch> {
        const CPU_SAMPLE_WINDOW_MS: u64 = 50;

        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        let cpu_usage =
            toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(CPU_SAMPLE_WINDOW_MS))
                .unwrap_or(0.0);
        metrics.push(Metric {
            name: "cpu_usage_percent".to_string(),
            value: MetricValue::Gauge(f64::from(cpu_usage)),
            labels: HashMap::new(),
            timestamp,
        });

        if let Ok(mem) = toadstool_sysmon::memory_info() {
            let memory_usage_percent = if mem.total > 0 {
                (mem.used as f64 / mem.total as f64) * 100.0
            } else {
                0.0
            };
            metrics.push(Metric {
                name: "memory_usage_percent".to_string(),
                value: MetricValue::Gauge(memory_usage_percent),
                labels: HashMap::new(),
                timestamp,
            });
        }

        let disks = toadstool_sysmon::disk_usage().unwrap_or_default();
        let (total_disk, used_disk) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
            (t + d.total_space, u + d.total_space - d.available_space)
        });
        let storage_usage_percent = if total_disk > 0 {
            (used_disk as f64 / total_disk as f64) * 100.0
        } else {
            0.0
        };
        metrics.push(Metric {
            name: "storage_usage_percent".to_string(),
            value: MetricValue::Gauge(storage_usage_percent),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "system".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "cpu".to_string(),
            "memory".to_string(),
            "storage".to_string(),
        ]
    }
}

/// Process metrics collector
pub struct ProcessMetricsCollector;

impl Default for ProcessMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessMetricsCollector {
    /// Create a new process metrics collector
    pub const fn new() -> Self {
        Self
    }
}

impl MetricsCollector for ProcessMetricsCollector {
    fn name(&self) -> &'static str {
        "process"
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        let process_count = toadstool_sysmon::process_count().unwrap_or(0);
        metrics.push(Metric {
            name: "process_count".to_string(),
            value: MetricValue::Gauge(process_count as f64),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "process".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["processes".to_string()]
    }
}

/// Network metrics collector
pub struct NetworkMetricsCollector;

impl Default for NetworkMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkMetricsCollector {
    /// Create a new network metrics collector
    pub const fn new() -> Self {
        Self
    }
}

impl MetricsCollector for NetworkMetricsCollector {
    fn name(&self) -> &'static str {
        "network"
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        let interfaces = toadstool_sysmon::network_stats().unwrap_or_default();
        let total_rx: u64 = interfaces.iter().map(|i| i.received).sum();
        let total_tx: u64 = interfaces.iter().map(|i| i.transmitted).sum();

        metrics.push(Metric {
            name: "network_rx_bytes_per_sec".to_string(),
            value: MetricValue::Gauge(total_rx as f64),
            labels: HashMap::new(),
            timestamp,
        });

        metrics.push(Metric {
            name: "network_tx_bytes_per_sec".to_string(),
            value: MetricValue::Gauge(total_tx as f64),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "network".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["network".to_string(), "throughput".to_string()]
    }
}

/// Finite dispatch for built-in [`MetricsCollector`] implementations.
pub enum MetricsCollectorDispatch {
    /// Host CPU, memory, and storage.
    System(SystemMetricsCollector),
    /// Process counts and related stats.
    Process(ProcessMetricsCollector),
    /// Network interface counters.
    Network(NetworkMetricsCollector),
}

impl MetricsCollector for MetricsCollectorDispatch {
    fn name(&self) -> &str {
        match self {
            Self::System(c) => c.name(),
            Self::Process(c) => c.name(),
            Self::Network(c) => c.name(),
        }
    }

    fn collect(&self) -> Result<MetricBatch> {
        match self {
            Self::System(c) => c.collect(),
            Self::Process(c) => c.collect(),
            Self::Network(c) => c.collect(),
        }
    }

    fn capabilities(&self) -> Vec<String> {
        match self {
            Self::System(c) => c.capabilities(),
            Self::Process(c) => c.capabilities(),
            Self::Network(c) => c.capabilities(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_collector_name() {
        let c = SystemMetricsCollector::new();
        assert_eq!(c.name(), "system");
    }

    #[test]
    fn system_collector_default() {
        let c = SystemMetricsCollector;
        assert_eq!(c.name(), "system");
    }

    #[test]
    fn system_collector_capabilities() {
        let c = SystemMetricsCollector::new();
        let caps = c.capabilities();
        assert!(caps.contains(&"cpu".to_string()));
        assert!(caps.contains(&"memory".to_string()));
        assert!(caps.contains(&"storage".to_string()));
    }

    #[test]
    fn system_collector_collect_returns_metrics() {
        let c = SystemMetricsCollector::new();
        let batch = c.collect().unwrap();
        assert_eq!(batch.source, "system");
        assert!(!batch.metrics.is_empty());
        assert!(
            batch
                .metrics
                .iter()
                .map(|m| m.name.as_str())
                .any(|x| x == "cpu_usage_percent")
        );
    }

    #[test]
    fn process_collector_name() {
        let c = ProcessMetricsCollector::new();
        assert_eq!(c.name(), "process");
    }

    #[test]
    fn process_collector_default() {
        let c = ProcessMetricsCollector;
        assert_eq!(c.name(), "process");
    }

    #[test]
    fn process_collector_capabilities() {
        let c = ProcessMetricsCollector::new();
        let caps = c.capabilities();
        assert!(caps.contains(&"processes".to_string()));
    }

    #[test]
    fn process_collector_collect_returns_metrics() {
        let c = ProcessMetricsCollector::new();
        let batch = c.collect().unwrap();
        assert_eq!(batch.source, "process");
        assert!(
            batch
                .metrics
                .iter()
                .map(|m| m.name.as_str())
                .any(|x| x == "process_count")
        );
    }

    #[test]
    fn network_collector_name() {
        let c = NetworkMetricsCollector::new();
        assert_eq!(c.name(), "network");
    }

    #[test]
    fn network_collector_default() {
        let c = NetworkMetricsCollector;
        assert_eq!(c.name(), "network");
    }

    #[test]
    fn network_collector_capabilities() {
        let c = NetworkMetricsCollector::new();
        let caps = c.capabilities();
        assert!(caps.contains(&"network".to_string()));
        assert!(caps.contains(&"throughput".to_string()));
    }

    #[test]
    fn network_collector_collect_returns_metrics() {
        let c = NetworkMetricsCollector::new();
        let batch = c.collect().unwrap();
        assert_eq!(batch.source, "network");
        let names: Vec<&str> = batch.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"network_rx_bytes_per_sec"));
        assert!(names.contains(&"network_tx_bytes_per_sec"));
    }

    #[test]
    fn dispatch_system_delegates() {
        let d = MetricsCollectorDispatch::System(SystemMetricsCollector::new());
        assert_eq!(d.name(), "system");
        assert!(!d.capabilities().is_empty());
        assert!(d.collect().is_ok());
    }

    #[test]
    fn dispatch_process_delegates() {
        let d = MetricsCollectorDispatch::Process(ProcessMetricsCollector::new());
        assert_eq!(d.name(), "process");
        assert!(!d.capabilities().is_empty());
        assert!(d.collect().is_ok());
    }

    #[test]
    fn dispatch_network_delegates() {
        let d = MetricsCollectorDispatch::Network(NetworkMetricsCollector::new());
        assert_eq!(d.name(), "network");
        assert!(!d.capabilities().is_empty());
        assert!(d.collect().is_ok());
    }
}
