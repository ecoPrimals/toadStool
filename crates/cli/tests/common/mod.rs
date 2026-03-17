// SPDX-License-Identifier: AGPL-3.0-only
//! Shared test helpers and mocks for CLI monitoring tests

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub struct MockMonitor {
    pub id: Uuid,
    pub running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub metrics: std::sync::Arc<tokio::sync::RwLock<Vec<Metric>>>,
    pub max_metrics: usize,
}

impl MockMonitor {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            metrics: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            max_metrics: 1000,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub async fn shutdown_gracefully(&mut self, _timeout: Duration) -> anyhow::Result<()> {
        self.stop()
    }

    #[allow(dead_code)]
    pub fn collect_metric(&mut self, _metric: Metric) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn cleanup(&mut self) {}

    pub fn get_metrics(&self) -> Vec<Metric> {
        vec![]
    }
}

pub struct MonitorConfig {
    pub interval_secs: u64,
    pub enabled: bool,
    pub collect_metrics: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub timestamp: std::time::SystemTime,
    pub labels: HashMap<String, String>,
}

#[allow(dead_code)]
pub struct AlertConfig {
    pub cpu_threshold: f64,
    pub memory_threshold: f64,
    pub disk_threshold: f64,
    pub enabled: bool,
}

#[allow(dead_code)]
pub struct Alert {
    pub severity: String,
    pub message: String,
    pub metric_name: String,
    pub value: f64,
    pub threshold: f64,
}

#[allow(dead_code)]
pub struct MonitorReport {
    pub duration: Duration,
    pub metric_count: usize,
    pub avg_cpu: f64,
    pub avg_memory: f64,
    pub alerts_triggered: usize,
}

#[allow(dead_code)]
pub struct MetricsSummary {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

pub async fn create_test_monitor() -> anyhow::Result<MockMonitor> {
    Ok(MockMonitor::new())
}

pub fn create_default_monitor_config() -> MonitorConfig {
    MonitorConfig {
        interval_secs: 30,
        enabled: true,
        collect_metrics: true,
    }
}

pub fn collect_cpu_metric() -> Metric {
    Metric {
        name: "cpu_percent".to_string(),
        value: 45.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn collect_memory_metric() -> Metric {
    Metric {
        name: "memory_bytes".to_string(),
        value: 1024.0 * 1024.0 * 1024.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn collect_disk_metric() -> Metric {
    Metric {
        name: "disk_bytes".to_string(),
        value: 10.0 * 1024.0 * 1024.0 * 1024.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn collect_network_rx_metric() -> Metric {
    Metric {
        name: "network_rx_bytes".to_string(),
        value: 1000000.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn collect_network_tx_metric() -> Metric {
    Metric {
        name: "network_tx_bytes".to_string(),
        value: 500000.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn create_test_metric() -> Metric {
    Metric {
        name: "test_metric".to_string(),
        value: 42.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    }
}

pub fn create_test_alert(metric: &str, value: f64) -> Alert {
    Alert {
        severity: "warning".to_string(),
        message: format!("{metric} above threshold"),
        metric_name: metric.to_string(),
        value,
        threshold: 80.0,
    }
}

pub fn _export_path() -> PathBuf {
    PathBuf::from("/tmp/metrics-export.json")
}
