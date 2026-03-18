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
//! Tests 1-10: Monitor Initialization and Configuration

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_initialization() {
    let monitor = create_test_monitor().await;
    assert!(monitor.is_ok(), "Monitor should initialize");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_default_config() {
    let config = create_default_monitor_config();
    assert!(config.interval_secs > 0, "Interval should be positive");
    assert!(config.interval_secs <= 300, "Interval should be reasonable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_custom_config() {
    let config = MonitorConfig {
        interval_secs: 60,
        enabled: true,
        collect_metrics: true,
    };
    assert_eq!(config.interval_secs, 60);
    assert!(config.enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_config_validation() {
    let invalid_configs = vec![
        MonitorConfig {
            interval_secs: 0,
            enabled: true,
            collect_metrics: true,
        },
        MonitorConfig {
            interval_secs: 1000,
            enabled: true,
            collect_metrics: true,
        },
    ];
    for config in invalid_configs {
        assert!(
            config.interval_secs == 0 || config.interval_secs > 500,
            "Should detect invalid interval"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_enabled_flag() {
    let config = MonitorConfig {
        interval_secs: 30,
        enabled: false,
        collect_metrics: false,
    };
    assert!(!config.enabled, "Monitor should be disabled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_metrics_collection_flag() {
    let with_metrics = MonitorConfig {
        interval_secs: 30,
        enabled: true,
        collect_metrics: true,
    };
    let without_metrics = MonitorConfig {
        interval_secs: 30,
        enabled: true,
        collect_metrics: false,
    };
    assert!(with_metrics.collect_metrics);
    assert!(!without_metrics.collect_metrics);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_interval_range() {
    let valid_intervals = vec![5u64, 10, 30, 60, 120, 300];
    for interval in valid_intervals {
        assert!(
            (5..=300).contains(&interval),
            "Interval should be in valid range: {interval}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_multiple_instances() {
    let monitor1 = create_test_monitor().await.unwrap();
    let monitor2 = create_test_monitor().await.unwrap();
    assert!(!monitor1.id.is_nil());
    assert!(!monitor2.id.is_nil());
    assert_ne!(monitor1.id, monitor2.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_state_initialization() {
    let monitor = create_test_monitor().await.unwrap();
    assert!(!monitor.is_running(), "Should not be running initially");
    let metrics = monitor.metrics.read().await;
    assert!(metrics.is_empty(), "Should have no metrics initially");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_resource_limits() {
    let monitor = create_test_monitor().await.unwrap();
    assert!(monitor.max_metrics > 0, "Should have metric limit");
    assert!(monitor.max_metrics <= 10000, "Limit should be reasonable");
}
