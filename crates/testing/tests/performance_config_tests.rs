// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Extended tests for performance test configuration

use std::time::Duration;
use toadstool_testing::performance::{
    PerformanceTestConfig, PerformanceTestManager, ResourceUsageMetrics,
};

#[test]
fn test_custom_config_creation() {
    let config = PerformanceTestConfig {
        test_name: "custom_test".to_string(),
        warm_up_iterations: 5,
        measurement_iterations: 50,
        concurrent_threads: 4,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: vec!["latency".to_string(), "throughput".to_string()],
    };

    assert_eq!(config.test_name, "custom_test");
    assert_eq!(config.warm_up_iterations, 5);
    assert_eq!(config.measurement_iterations, 50);
    assert_eq!(config.concurrent_threads, 4);
    assert!(!config.memory_profiling);
    assert!(!config.cpu_profiling);
    assert_eq!(config.custom_metrics.len(), 2);
}

#[test]
fn test_config_with_no_metrics() {
    let config = PerformanceTestConfig {
        test_name: "no_metrics".to_string(),
        custom_metrics: vec![],
        ..Default::default()
    };

    assert!(config.custom_metrics.is_empty());
    assert!(config.memory_profiling); // default is true
    assert!(config.cpu_profiling); // default is true
}

#[test]
fn test_config_with_many_custom_metrics() {
    let metrics: Vec<String> = (0..10).map(|i| format!("metric_{i}")).collect();
    let config = PerformanceTestConfig {
        test_name: "many_metrics".to_string(),
        custom_metrics: metrics.clone(),
        ..Default::default()
    };

    assert_eq!(config.custom_metrics.len(), 10);
    assert_eq!(config.custom_metrics[0], "metric_0");
    assert_eq!(config.custom_metrics[9], "metric_9");
}

#[test]
fn test_config_extreme_iterations() {
    let config = PerformanceTestConfig {
        test_name: "extreme".to_string(),
        warm_up_iterations: 0,
        measurement_iterations: 1,
        ..Default::default()
    };

    assert_eq!(config.warm_up_iterations, 0);
    assert_eq!(config.measurement_iterations, 1);
}

#[test]
fn test_config_high_concurrency() {
    let config = PerformanceTestConfig {
        test_name: "high_concurrency".to_string(),
        concurrent_threads: 128,
        ..Default::default()
    };

    assert_eq!(config.concurrent_threads, 128);
}

#[test]
fn test_manager_creation_with_default_config() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);
    drop(manager); // Verify creation succeeds
}

#[test]
fn test_manager_creation_with_custom_config() {
    let config = PerformanceTestConfig {
        test_name: "custom_manager".to_string(),
        warm_up_iterations: 3,
        measurement_iterations: 10,
        concurrent_threads: 2,
        memory_profiling: true,
        cpu_profiling: true,
        custom_metrics: vec!["test_metric".to_string()],
    };

    let manager = PerformanceTestManager::new(config);
    drop(manager);
}

#[test]
fn test_resource_usage_metrics_creation() {
    let metrics = ResourceUsageMetrics {
        peak_memory_mb: 100,
        average_memory_mb: 80,
        peak_cpu_percent: 75.5,
        average_cpu_percent: 50.0,
        disk_io_mb: 1024,
        network_io_mb: 512,
        context_switches: 1000,
    };

    assert_eq!(metrics.peak_memory_mb, 100);
    assert_eq!(metrics.average_memory_mb, 80);
    assert!((metrics.peak_cpu_percent - 75.5).abs() < f32::EPSILON);
    assert!((metrics.average_cpu_percent - 50.0).abs() < f32::EPSILON);
    assert_eq!(metrics.disk_io_mb, 1024);
    assert_eq!(metrics.network_io_mb, 512);
    assert_eq!(metrics.context_switches, 1000);
}

#[test]
fn test_resource_usage_metrics_zero_values() {
    let metrics = ResourceUsageMetrics::default();
    assert_eq!(metrics.peak_memory_mb, 0);
    assert_eq!(metrics.average_memory_mb, 0);
    assert_eq!(metrics.peak_cpu_percent, 0.0);
    assert_eq!(metrics.average_cpu_percent, 0.0);
    assert_eq!(metrics.disk_io_mb, 0);
    assert_eq!(metrics.network_io_mb, 0);
    assert_eq!(metrics.context_switches, 0);
}

#[test]
fn test_resource_usage_metrics_clone() {
    let metrics = ResourceUsageMetrics {
        peak_memory_mb: 200,
        average_memory_mb: 150,
        peak_cpu_percent: 90.0,
        average_cpu_percent: 75.0,
        disk_io_mb: 2048,
        network_io_mb: 1024,
        context_switches: 5000,
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.peak_memory_mb, cloned.peak_memory_mb);
    assert_eq!(metrics.average_memory_mb, cloned.average_memory_mb);
    assert_eq!(metrics.disk_io_mb, cloned.disk_io_mb);
}

#[test]
fn test_config_builder_pattern() {
    let config = PerformanceTestConfig {
        test_name: "builder_test".to_string(),
        ..Default::default()
    };

    assert_eq!(config.test_name, "builder_test");
    assert_eq!(config.warm_up_iterations, 10); // default
    assert_eq!(config.measurement_iterations, 100); // default
}

#[test]
fn test_config_no_profiling() {
    let config = PerformanceTestConfig {
        test_name: "no_profiling".to_string(),
        memory_profiling: false,
        cpu_profiling: false,
        ..Default::default()
    };

    assert!(!config.memory_profiling);
    assert!(!config.cpu_profiling);
}

#[test]
fn test_config_long_test_name() {
    let long_name = "a".repeat(1000);
    let config = PerformanceTestConfig {
        test_name: long_name.clone(),
        ..Default::default()
    };

    assert_eq!(config.test_name.len(), 1000);
    assert_eq!(config.test_name, long_name);
}

#[test]
fn test_config_special_characters_in_name() {
    let config = PerformanceTestConfig {
        test_name: "test::with::colons::and-dashes".to_string(),
        ..Default::default()
    };

    assert_eq!(config.test_name, "test::with::colons::and-dashes");
}

#[test]
fn test_multiple_managers_same_config() {
    let config = PerformanceTestConfig::default();
    let manager1 = PerformanceTestManager::new(config.clone());
    let manager2 = PerformanceTestManager::new(config);

    drop(manager1);
    drop(manager2);
}

#[test]
fn test_metrics_high_values() {
    let metrics = ResourceUsageMetrics {
        peak_memory_mb: u32::MAX,
        average_memory_mb: u32::MAX / 2,
        peak_cpu_percent: 100.0,
        average_cpu_percent: 99.9,
        disk_io_mb: u64::MAX,
        network_io_mb: u64::MAX - 1,
        context_switches: u64::MAX,
    };

    assert_eq!(metrics.peak_memory_mb, u32::MAX);
    assert_eq!(metrics.disk_io_mb, u64::MAX);
}

#[tokio::test]
async fn test_manager_with_zero_iterations() {
    let config = PerformanceTestConfig {
        test_name: "zero_iterations".to_string(),
        warm_up_iterations: 0,
        measurement_iterations: 1, // At least 1 for measurement
        ..Default::default()
    };

    let manager = PerformanceTestManager::new(config);
    let result = manager
        .benchmark(|| async { Ok(()) })
        .await
        .expect("Should succeed with minimal iterations");

    assert_eq!(result.test_name, "zero_iterations");
    assert_eq!(result.iterations, 1);
}

#[tokio::test]
async fn test_manager_simple_async_benchmark() {
    let config = PerformanceTestConfig {
        test_name: "simple_async".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 3,
        memory_profiling: false,
        cpu_profiling: false,
        ..Default::default()
    };

    let manager = PerformanceTestManager::new(config);
    let result = manager
        .benchmark(|| async {
            // Minimal CPU work ensures non-zero wall-clock measurement
            // without blocking the async executor.
            let _ = (0..100u64).fold(0u64, |a, b| a.wrapping_add(b));
            tokio::task::yield_now().await;
            Ok(())
        })
        .await
        .expect("Benchmark should succeed");

    assert_eq!(result.test_name, "simple_async");
    assert_eq!(result.iterations, 3);
    assert!(result.total_duration > Duration::ZERO);
    assert!(result.average_duration > Duration::ZERO);
}

#[tokio::test]
async fn test_generate_empty_report() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);

    let report = manager.generate_report().await;
    assert_eq!(report.total_benchmarks, 0);
    assert!(report.results.is_empty());
}
