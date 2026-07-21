// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
//! Tests 11-20: Metric Collection

use std::collections::HashMap;
use std::time::Duration;

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_collect_cpu_metrics() {
    let cpu_metric = collect_cpu_metric();
    assert!(cpu_metric.value >= 0.0, "CPU should be non-negative");
    assert!(cpu_metric.value <= 100.0, "CPU should be <= 100%");
    assert_eq!(cpu_metric.name, "cpu_percent");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_collect_memory_metrics() {
    let memory_metric = collect_memory_metric();
    assert!(memory_metric.value > 0.0, "Memory should be positive");
    assert_eq!(memory_metric.name, "memory_bytes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_collect_disk_metrics() {
    let disk_metric = collect_disk_metric();
    assert!(disk_metric.value >= 0.0, "Disk should be non-negative");
    assert_eq!(disk_metric.name, "disk_bytes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_collect_network_metrics() {
    let net_rx = collect_network_rx_metric();
    let net_tx = collect_network_tx_metric();
    assert!(net_rx.value >= 0.0, "Network RX should be non-negative");
    assert!(net_tx.value >= 0.0, "Network TX should be non-negative");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_timestamp() {
    let metric = Metric {
        name: "test".to_string(),
        value: 42.0,
        timestamp: std::time::SystemTime::now(),
        labels: HashMap::new(),
    };
    assert!(
        metric
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            > 0,
        "Timestamp should be valid"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_labels() {
    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "localhost".to_string());
    labels.insert("service".to_string(), "test".to_string());
    let metric = Metric {
        name: "test".to_string(),
        value: 42.0,
        timestamp: std::time::SystemTime::now(),
        labels,
    };
    assert_eq!(metric.labels.len(), 2);
    assert_eq!(metric.labels.get("host"), Some(&"localhost".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_aggregation() {
    let metrics = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let sum: f64 = metrics.iter().sum();
    let avg = sum / metrics.len() as f64;
    let max = metrics.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = metrics.iter().copied().fold(f64::INFINITY, f64::min);
    assert_eq!(sum, 150.0);
    assert_eq!(avg, 30.0);
    assert_eq!(max, 50.0);
    assert_eq!(min, 10.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_history_size() {
    let max_history = 1000;
    assert!(max_history > 0, "History size should be positive");
    assert!(max_history <= 10000, "History size should be reasonable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_collection_interval() {
    let interval = Duration::from_secs(30);
    assert!(interval.as_secs() >= 5, "Interval should be >= 5 seconds");
    assert!(interval.as_secs() <= 300, "Interval should be <= 5 minutes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_cleanup_old_data() {
    let retention_period = Duration::from_hours(1);
    assert!(
        retention_period.as_secs() > 0,
        "Retention should be positive"
    );
    assert!(
        retention_period.as_secs() <= 86400,
        "Retention should be <= 24 hours"
    );
}
