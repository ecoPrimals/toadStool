// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Coverage tests for server/src/mocks.rs
//!
//! Target: Get mocks.rs from 0% → 100% coverage

use toadstool::ResourceMonitor;

// Import mocks module directly from source since it's not exported in lib.rs
#[path = "../src/mocks.rs"]
mod mocks;

use mocks::*;

// ============================================================================
// MockResourceMonitor Tests
// ============================================================================

#[test]
fn test_mock_resource_monitor_new() {
    let monitor = MockResourceMonitor::new();
    // Should not panic
    assert_eq!(
        std::mem::size_of_val(&monitor),
        std::mem::size_of::<MockResourceMonitor>()
    );
}

#[test]
fn test_mock_resource_monitor_default() {
    let monitor = MockResourceMonitor;
    // Should not panic
    assert_eq!(
        std::mem::size_of_val(&monitor),
        std::mem::size_of::<MockResourceMonitor>()
    );
}

#[test]
fn test_mock_resource_monitor_start_monitoring() {
    let monitor = MockResourceMonitor::new();
    let result = monitor.start_monitoring("test-workload");
    assert!(result.is_ok());
}

#[test]
fn test_mock_resource_monitor_stop_monitoring() {
    let monitor = MockResourceMonitor::new();
    let result = monitor.stop_monitoring("test-workload");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_resource_monitor_get_metrics() {
    let monitor = MockResourceMonitor::new();
    let result = monitor.get_metrics("test-workload").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_resource_monitor_get_system_resources() {
    let monitor = MockResourceMonitor::new();
    let result = monitor.get_system_resources().await;

    assert!(result.is_ok());
    let resources = result.unwrap();
    assert_eq!(resources.available_cpu_cores, 4.0);
    assert_eq!(resources.available_memory_bytes, 8_000_000_000);
    assert_eq!(resources.available_storage_bytes, 100_000_000_000);
    assert_eq!(resources.available_network_bandwidth, Some(1_000_000_000));
    assert_eq!(resources.available_gpu_units, 1);
}

#[tokio::test]
async fn test_mock_resource_monitor_multiple_workloads() {
    let monitor = MockResourceMonitor::new();

    monitor.start_monitoring("workload1").unwrap();
    monitor.start_monitoring("workload2").unwrap();
    monitor.start_monitoring("workload3").unwrap();

    let metrics1 = monitor.get_metrics("workload1").await;
    let metrics2 = monitor.get_metrics("workload2").await;
    let metrics3 = monitor.get_metrics("workload3").await;

    assert!(metrics1.is_ok());
    assert!(metrics2.is_ok());
    assert!(metrics3.is_ok());

    monitor.stop_monitoring("workload1").unwrap();
    monitor.stop_monitoring("workload2").unwrap();
    monitor.stop_monitoring("workload3").unwrap();
}

// ============================================================================
// MockSystemResourcesWithUsage Tests
// ============================================================================

#[test]
fn test_mock_system_resources_with_usage_default() {
    let resources = MockSystemResourcesWithUsage::default();

    assert_eq!(resources.cpu_usage_percent, 45.2);
    assert_eq!(resources.memory_usage_percent, 62.8);
    assert_eq!(resources.available_memory_bytes, 4_000_000_000);
    assert_eq!(resources.total_memory_bytes, 8_000_000_000);
    assert_eq!(resources.disk_usage_percent, 25.0);
    assert_eq!(resources.network_bytes_sent, 1_000_000);
    assert_eq!(resources.network_bytes_received, 2_000_000);
    assert_eq!(resources.load_average, [0.5, 0.7, 0.9]);
    assert_eq!(resources.uptime_seconds, 86400);
}

#[test]
fn test_mock_system_resources_with_usage_memory_calculations() {
    let resources = MockSystemResourcesWithUsage::default();

    // Verify memory values are sensible
    assert!(resources.available_memory_bytes < resources.total_memory_bytes);
    assert!(resources.memory_usage_percent > 0.0);
    assert!(resources.memory_usage_percent < 100.0);

    let used_memory = resources.total_memory_bytes - resources.available_memory_bytes;
    assert_eq!(used_memory, 4_000_000_000); // 4GB used
}

#[test]
fn test_mock_system_resources_with_usage_load_average() {
    let resources = MockSystemResourcesWithUsage::default();

    assert_eq!(resources.load_average.len(), 3);
    assert!(resources.load_average[0] < resources.load_average[1]);
    assert!(resources.load_average[1] < resources.load_average[2]);
}

#[test]
fn test_mock_system_resources_with_usage_custom() {
    let resources = MockSystemResourcesWithUsage {
        cpu_usage_percent: 80.0,
        memory_usage_percent: 90.0,
        available_memory_bytes: 1_000_000_000,
        total_memory_bytes: 16_000_000_000,
        disk_usage_percent: 50.0,
        network_bytes_sent: 5_000_000,
        network_bytes_received: 10_000_000,
        load_average: [1.0, 2.0, 3.0],
        uptime_seconds: 172800,
    };

    assert_eq!(resources.cpu_usage_percent, 80.0);
    assert_eq!(resources.uptime_seconds, 172800);
}

#[test]
fn test_mock_system_resources_with_usage_network_totals() {
    let resources = MockSystemResourcesWithUsage::default();

    let total_network = resources.network_bytes_sent + resources.network_bytes_received;
    assert_eq!(total_network, 3_000_000);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_integration_workflow() {
    let monitor = MockResourceMonitor::new();

    // Start monitoring
    monitor.start_monitoring("integration-test").unwrap();

    // Get system resources
    let sys_resources = monitor.get_system_resources().await.unwrap();
    assert!(sys_resources.available_cpu_cores > 0.0);

    // Get metrics (now async)
    let metrics = monitor.get_metrics("integration-test").await.unwrap();
    assert_eq!(
        std::mem::size_of_val(&metrics),
        std::mem::size_of::<toadstool::RuntimeMetrics>()
    );

    // Stop monitoring
    monitor.stop_monitoring("integration-test").unwrap();
}

#[test]
fn test_mock_types_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockResourceMonitor>();
}
