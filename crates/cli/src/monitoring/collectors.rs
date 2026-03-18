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

    #[allow(clippy::cast_precision_loss)]
    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        let cpu_usage =
            toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(50)).unwrap_or(0.0);
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

    #[allow(clippy::cast_precision_loss)]
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

    #[allow(clippy::cast_precision_loss)]
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
