//! Metrics collectors - system, process, and network data collection

use anyhow::Result;
use std::collections::HashMap;
use sysinfo::{Disks, Networks, System};

use crate::monitoring::types::{Metric, MetricBatch, MetricValue};

/// Metrics collection interface
pub trait MetricsCollector: Send + Sync {
    fn name(&self) -> &str;
    fn collect(&self) -> Result<MetricBatch>;
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
    pub fn new() -> Self {
        Self
    }
}

impl MetricsCollector for SystemMetricsCollector {
    fn name(&self) -> &'static str {
        "system"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        // System metrics (CPU, memory)
        let mut system = System::new_all();
        system.refresh_all();

        // Real CPU usage (sysinfo 0.30+ API)
        let cpu_usage = system.global_cpu_info().cpu_usage();
        metrics.push(Metric {
            name: "cpu_usage_percent".to_string(),
            value: MetricValue::Gauge(f64::from(cpu_usage)),
            labels: HashMap::new(),
            timestamp,
        });

        // Real memory usage
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_percent = if total_memory > 0 {
            (used_memory as f64 / total_memory as f64) * 100.0
        } else {
            0.0
        };
        metrics.push(Metric {
            name: "memory_usage_percent".to_string(),
            value: MetricValue::Gauge(memory_usage_percent),
            labels: HashMap::new(),
            timestamp,
        });

        // Real storage usage (sysinfo 0.30+ uses separate Disks struct)
        let disks = Disks::new_with_refreshed_list();
        let mut total_disk: u64 = 0;
        let mut used_disk: u64 = 0;
        for disk in disks.iter() {
            total_disk += disk.total_space();
            used_disk += disk.total_space() - disk.available_space();
        }
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
    pub fn new() -> Self {
        Self
    }
}

impl MetricsCollector for ProcessMetricsCollector {
    fn name(&self) -> &'static str {
        "process"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();
        let mut system = System::new_all();
        system.refresh_processes();

        // Real process count
        let process_count = system.processes().len();
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
    pub fn new() -> Self {
        Self
    }
}

impl MetricsCollector for NetworkMetricsCollector {
    fn name(&self) -> &'static str {
        "network"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = std::time::SystemTime::now();
        let mut metrics = Vec::new();

        // sysinfo 0.30+ uses separate Networks struct
        let networks = Networks::new_with_refreshed_list();

        // Real network throughput
        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;
        for (_interface_name, network) in networks.iter() {
            total_rx += network.received();
            total_tx += network.transmitted();
        }

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
