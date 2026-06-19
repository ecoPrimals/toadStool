// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Coverage tests for toadstool-testing mock infrastructure.
//!
//! Validates that the canonical `MockResourceMonitor` from `toadstool-testing`
//! works correctly in server test contexts.

use toadstool::ResourceMonitor;
use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;

#[test]
fn test_mock_resource_monitor_successful() {
    let monitor = MockResourceMonitor::new_successful();
    assert!(monitor.start_monitoring("test-workload").is_ok());
    assert!(monitor.stop_monitoring("test-workload").is_ok());
}

#[tokio::test]
async fn test_mock_resource_monitor_get_metrics() {
    let monitor = MockResourceMonitor::new_successful();
    let result = monitor.get_metrics("test-workload").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_resource_monitor_get_system_resources() {
    let monitor = MockResourceMonitor::new_successful();
    let result = monitor.get_system_resources().await;

    assert!(result.is_ok());
    let resources = result.unwrap();
    assert!(resources.available_cpu_cores > 0.0);
}

#[tokio::test]
async fn test_mock_resource_monitor_multiple_workloads() {
    let monitor = MockResourceMonitor::new_successful();

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

#[tokio::test]
async fn test_mock_resource_monitor_limit_violations() {
    let monitor = MockResourceMonitor::new_limit_violations();

    assert!(monitor.start_monitoring("test-workload").is_ok());
    let metrics = monitor.get_metrics("test-workload").await.unwrap();
    assert!(metrics.cpu.usage_percent > 90.0);

    let system_resources = monitor.get_system_resources().await.unwrap();
    assert!(system_resources.available_cpu_cores < 4.0);
    assert!(monitor.stop_monitoring("test-workload").is_ok());
}

#[tokio::test]
async fn test_mock_resource_monitor_failure_mode() {
    let monitor = MockResourceMonitor::new_monitoring_failure();
    assert!(monitor.start_monitoring("test-workload").is_err());
    assert!(monitor.get_metrics("test-workload").await.is_err());
    assert!(monitor.stop_monitoring("test-workload").is_err());
    assert!(monitor.get_system_resources().await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mock_integration_workflow() {
    let monitor = MockResourceMonitor::new_successful();

    monitor.start_monitoring("integration-test").unwrap();

    let sys_resources = monitor.get_system_resources().await.unwrap();
    assert!(sys_resources.available_cpu_cores > 0.0);

    let metrics = monitor.get_metrics("integration-test").await.unwrap();
    assert_eq!(
        std::mem::size_of_val(&metrics),
        std::mem::size_of::<toadstool::RuntimeMetrics>()
    );

    monitor.stop_monitoring("integration-test").unwrap();
}

#[test]
fn test_mock_types_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockResourceMonitor>();
}
