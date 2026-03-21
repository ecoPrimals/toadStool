// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Comprehensive tests for the metrics collector module

use std::collections::HashMap;
use std::time::Duration;
use toadstool_distributed::metrics::*;

#[test]
fn test_local_metrics_default() {
    let metrics = LocalMetrics::default();

    assert_eq!(metrics.active_jobs, 0);
    assert_eq!(metrics.total_processed, 0);
    assert_eq!(metrics.success_rate, 0.0);
    assert_eq!(metrics.average_execution_time, Duration::from_secs(0));
}

#[test]
fn test_local_metrics_creation() {
    let metrics = LocalMetrics {
        active_jobs: 5,
        total_processed: 100,
        success_rate: 0.95,
        average_execution_time: Duration::from_millis(250),
    };

    assert_eq!(metrics.active_jobs, 5);
    assert_eq!(metrics.total_processed, 100);
    assert_eq!(metrics.success_rate, 0.95);
    assert_eq!(metrics.average_execution_time, Duration::from_millis(250));
}

#[test]
fn test_local_metrics_clone() {
    let metrics = LocalMetrics {
        active_jobs: 10,
        total_processed: 500,
        success_rate: 0.99,
        average_execution_time: Duration::from_millis(100),
    };

    let cloned = metrics.clone();
    assert_eq!(cloned.active_jobs, metrics.active_jobs);
    assert_eq!(cloned.total_processed, metrics.total_processed);
    assert_eq!(cloned.success_rate, metrics.success_rate);
    assert_eq!(
        cloned.average_execution_time,
        metrics.average_execution_time
    );
}

#[test]
fn test_local_metrics_debug() {
    let metrics = LocalMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("LocalMetrics"));
}

#[test]
fn test_network_metrics_default() {
    let metrics = NetworkMetrics::default();

    assert_eq!(metrics.active_network_jobs, 0);
    assert_eq!(metrics.network_utilization, 0.0);
    assert_eq!(metrics.average_latency, Duration::from_millis(0));
}

#[test]
fn test_network_metrics_creation() {
    let metrics = NetworkMetrics {
        active_network_jobs: 15,
        network_utilization: 0.75,
        average_latency: Duration::from_millis(50),
    };

    assert_eq!(metrics.active_network_jobs, 15);
    assert_eq!(metrics.network_utilization, 0.75);
    assert_eq!(metrics.average_latency, Duration::from_millis(50));
}

#[test]
fn test_network_metrics_high_utilization() {
    let metrics = NetworkMetrics {
        active_network_jobs: 100,
        network_utilization: 0.95,
        average_latency: Duration::from_millis(200),
    };

    assert!(metrics.network_utilization > 0.9);
    assert!(metrics.average_latency > Duration::from_millis(100));
}

#[test]
fn test_network_metrics_clone() {
    let metrics = NetworkMetrics {
        active_network_jobs: 25,
        network_utilization: 0.50,
        average_latency: Duration::from_millis(75),
    };

    let cloned = metrics.clone();
    assert_eq!(cloned.active_network_jobs, metrics.active_network_jobs);
    assert_eq!(cloned.network_utilization, metrics.network_utilization);
    assert_eq!(cloned.average_latency, metrics.average_latency);
}

#[test]
fn test_network_metrics_debug() {
    let metrics = NetworkMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("NetworkMetrics"));
}

#[test]
fn test_ecosystem_metrics_default() {
    let metrics = EcosystemMetrics::default();

    assert_eq!(metrics.active_ecosystem_jobs, 0);
    assert!(metrics.ecosystem_service_calls.is_empty());
    assert!(metrics.ecosystem_success_rates.is_empty());
}

#[test]
fn test_ecosystem_metrics_creation() {
    let mut service_calls = HashMap::new();
    service_calls.insert("songbird".to_string(), 50);
    service_calls.insert("beardog".to_string(), 30);

    let mut success_rates = HashMap::new();
    success_rates.insert("songbird".to_string(), 0.98);
    success_rates.insert("beardog".to_string(), 0.95);

    let metrics = EcosystemMetrics {
        active_ecosystem_jobs: 5,
        ecosystem_service_calls: service_calls.clone(),
        ecosystem_success_rates: success_rates.clone(),
    };

    assert_eq!(metrics.active_ecosystem_jobs, 5);
    assert_eq!(metrics.ecosystem_service_calls.len(), 2);
    assert_eq!(metrics.ecosystem_success_rates.len(), 2);
    assert_eq!(metrics.ecosystem_service_calls.get("songbird"), Some(&50));
    assert_eq!(metrics.ecosystem_success_rates.get("beardog"), Some(&0.95));
}

#[test]
fn test_ecosystem_metrics_add_service() {
    let mut metrics = EcosystemMetrics::default();

    metrics
        .ecosystem_service_calls
        .insert("squirrel".to_string(), 10);
    metrics
        .ecosystem_success_rates
        .insert("squirrel".to_string(), 0.92);

    assert_eq!(metrics.ecosystem_service_calls.len(), 1);
    assert_eq!(metrics.ecosystem_success_rates.len(), 1);
}

#[test]
fn test_ecosystem_metrics_multiple_services() {
    let mut service_calls = HashMap::new();
    service_calls.insert("songbird".to_string(), 100);
    service_calls.insert("beardog".to_string(), 80);
    service_calls.insert("squirrel".to_string(), 60);
    service_calls.insert("nestgate".to_string(), 40);

    let metrics = EcosystemMetrics {
        active_ecosystem_jobs: 10,
        ecosystem_service_calls: service_calls.clone(),
        ecosystem_success_rates: HashMap::new(),
    };

    assert_eq!(metrics.ecosystem_service_calls.len(), 4);
    assert!(metrics.ecosystem_service_calls.contains_key("songbird"));
    assert!(metrics.ecosystem_service_calls.contains_key("beardog"));
}

#[test]
fn test_ecosystem_metrics_clone() {
    let mut service_calls = HashMap::new();
    service_calls.insert("test".to_string(), 42);

    let metrics = EcosystemMetrics {
        active_ecosystem_jobs: 8,
        ecosystem_service_calls: service_calls,
        ..Default::default()
    };

    let cloned = metrics.clone();
    assert_eq!(cloned.active_ecosystem_jobs, metrics.active_ecosystem_jobs);
    assert_eq!(
        cloned.ecosystem_service_calls.len(),
        metrics.ecosystem_service_calls.len()
    );
}

#[test]
fn test_ecosystem_metrics_debug() {
    let metrics = EcosystemMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("EcosystemMetrics"));
}

#[test]
fn test_recursive_hosting_metrics_default() {
    let metrics = RecursiveHostingMetrics::default();

    assert_eq!(metrics.active_child_instances, 0);
    assert_eq!(metrics.total_child_instances_created, 0);
    assert_eq!(metrics.child_instance_success_rate, 0.0);
}

#[test]
fn test_recursive_hosting_metrics_creation() {
    let metrics = RecursiveHostingMetrics {
        active_child_instances: 5,
        total_child_instances_created: 50,
        child_instance_success_rate: 0.96,
    };

    assert_eq!(metrics.active_child_instances, 5);
    assert_eq!(metrics.total_child_instances_created, 50);
    assert_eq!(metrics.child_instance_success_rate, 0.96);
}

#[test]
fn test_recursive_hosting_metrics_high_count() {
    let metrics = RecursiveHostingMetrics {
        active_child_instances: 100,
        total_child_instances_created: 1000,
        child_instance_success_rate: 0.99,
    };

    assert_eq!(metrics.active_child_instances, 100);
    assert!(metrics.total_child_instances_created >= metrics.active_child_instances);
    assert!(metrics.child_instance_success_rate >= 0.90);
}

#[test]
fn test_recursive_hosting_metrics_clone() {
    let metrics = RecursiveHostingMetrics {
        active_child_instances: 10,
        total_child_instances_created: 100,
        child_instance_success_rate: 0.95,
    };

    let cloned = metrics.clone();
    assert_eq!(
        cloned.active_child_instances,
        metrics.active_child_instances
    );
    assert_eq!(
        cloned.total_child_instances_created,
        metrics.total_child_instances_created
    );
    assert_eq!(
        cloned.child_instance_success_rate,
        metrics.child_instance_success_rate
    );
}

#[test]
fn test_recursive_hosting_metrics_debug() {
    let metrics = RecursiveHostingMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("RecursiveHostingMetrics"));
}

#[test]
fn test_universal_metrics_collector_new() {
    let collector = UniversalMetricsCollector::new();
    // Should create without panicking
    drop(collector);
}

#[test]
fn test_universal_metrics_collector_default() {
    let collector = UniversalMetricsCollector::default();
    // Should create without panicking
    drop(collector);
}

#[test]
fn test_universal_metrics_collector_creation_consistency() {
    let collector1 = UniversalMetricsCollector::new();
    let collector2 = UniversalMetricsCollector::default();

    // Both should create valid instances
    drop(collector1);
    drop(collector2);
}

#[test]
fn test_local_metrics_with_zero_values() {
    let metrics = LocalMetrics {
        active_jobs: 0,
        total_processed: 0,
        success_rate: 0.0,
        average_execution_time: Duration::from_secs(0),
    };

    assert_eq!(metrics.active_jobs, 0);
    assert_eq!(metrics.success_rate, 0.0);
}

#[test]
fn test_local_metrics_with_max_values() {
    let metrics = LocalMetrics {
        active_jobs: u64::MAX,
        total_processed: u64::MAX,
        success_rate: 1.0,
        average_execution_time: Duration::from_secs(3600),
    };

    assert_eq!(metrics.active_jobs, u64::MAX);
    assert_eq!(metrics.success_rate, 1.0);
}

#[test]
fn test_network_metrics_with_zero_latency() {
    let metrics = NetworkMetrics {
        active_network_jobs: 50,
        network_utilization: 0.8,
        average_latency: Duration::from_nanos(0),
    };

    assert_eq!(metrics.average_latency, Duration::from_nanos(0));
}

#[test]
fn test_network_metrics_with_microsecond_latency() {
    let metrics = NetworkMetrics {
        active_network_jobs: 20,
        network_utilization: 0.5,
        average_latency: Duration::from_micros(500),
    };

    assert_eq!(metrics.average_latency, Duration::from_micros(500));
    assert!(metrics.average_latency < Duration::from_millis(1));
}

#[test]
fn test_ecosystem_metrics_empty_hashmap() {
    let metrics = EcosystemMetrics {
        active_ecosystem_jobs: 5,
        ecosystem_service_calls: HashMap::new(),
        ecosystem_success_rates: HashMap::new(),
    };

    assert!(metrics.ecosystem_service_calls.is_empty());
    assert!(metrics.ecosystem_success_rates.is_empty());
    assert_eq!(metrics.active_ecosystem_jobs, 5);
}

#[test]
fn test_ecosystem_metrics_single_service() {
    let mut service_calls = HashMap::new();
    service_calls.insert("single_service".to_string(), 1);

    let metrics = EcosystemMetrics {
        active_ecosystem_jobs: 1,
        ecosystem_service_calls: service_calls,
        ecosystem_success_rates: HashMap::new(),
    };

    assert_eq!(metrics.ecosystem_service_calls.len(), 1);
    assert_eq!(
        metrics.ecosystem_service_calls.get("single_service"),
        Some(&1)
    );
}

#[test]
fn test_recursive_hosting_metrics_perfect_success_rate() {
    let metrics = RecursiveHostingMetrics {
        active_child_instances: 50,
        total_child_instances_created: 50,
        child_instance_success_rate: 1.0,
    };

    assert_eq!(metrics.child_instance_success_rate, 1.0);
}

#[test]
fn test_recursive_hosting_metrics_zero_success_rate() {
    let metrics = RecursiveHostingMetrics {
        active_child_instances: 0,
        total_child_instances_created: 50,
        child_instance_success_rate: 0.0,
    };

    assert_eq!(metrics.child_instance_success_rate, 0.0);
}

#[test]
fn test_duration_comparison_in_metrics() {
    let metrics1 = LocalMetrics {
        active_jobs: 10,
        total_processed: 100,
        success_rate: 0.9,
        average_execution_time: Duration::from_millis(100),
    };

    let metrics2 = LocalMetrics {
        active_jobs: 10,
        total_processed: 100,
        success_rate: 0.9,
        average_execution_time: Duration::from_millis(200),
    };

    assert!(metrics1.average_execution_time < metrics2.average_execution_time);
}

#[test]
fn test_success_rate_boundaries() {
    let metrics_min = LocalMetrics {
        active_jobs: 100,
        total_processed: 1000,
        success_rate: 0.0,
        average_execution_time: Duration::from_millis(50),
    };

    let metrics_max = LocalMetrics {
        active_jobs: 100,
        total_processed: 1000,
        success_rate: 1.0,
        average_execution_time: Duration::from_millis(50),
    };

    assert_eq!(metrics_min.success_rate, 0.0);
    assert_eq!(metrics_max.success_rate, 1.0);
}
