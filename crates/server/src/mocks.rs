// SPDX-License-Identifier: AGPL-3.0-only
//! Mock implementations for testing
//!
//! ⚠️ **TEST-ONLY MODULE**
//! These mocks are for testing infrastructure only and should never be used in production.
//!
//! This module is gated with `#[cfg(test)]` in `lib.rs` — it is never compiled into production builds.

#[cfg(test)]
use std::future::Future;
#[cfg(test)]
use std::pin::Pin;
#[cfg(test)]
use toadstool::{ResourceMonitor, RuntimeMetrics, SystemResources, ToadStoolResult};

/// Mock resource monitor for testing
#[cfg(test)]
pub struct MockResourceMonitor;

#[cfg(test)]
impl Default for MockResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MockResourceMonitor {
    /// Create a new mock resource monitor
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
impl ResourceMonitor for MockResourceMonitor {
    fn start_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    fn stop_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    fn get_metrics(
        &self,
        _workload_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async move { Ok(RuntimeMetrics::default()) })
    }

    fn get_system_resources(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>> {
        Box::pin(async move {
            Ok(SystemResources {
                available_cpu_cores: 4.0,
                available_memory_bytes: 8_000_000_000,
                available_storage_bytes: 100_000_000_000,
                available_network_bandwidth: Some(1_000_000_000),
                available_gpu_units: 1,
                cpu_usage_percent: 25.0,
                memory_usage_percent: 50.0,
                total_cpu_cores: 8,
                total_memory_bytes: 16_000_000_000,
            })
        })
    }
}

/// Mock system resources for testing (with usage metrics)
#[cfg(test)]
pub struct MockSystemResourcesWithUsage {
    /// CPU usage percentage.
    pub cpu_usage_percent: f64,
    /// Memory usage percentage.
    pub memory_usage_percent: f64,
    /// Available memory in bytes.
    pub available_memory_bytes: u64,
    /// Total memory in bytes.
    pub total_memory_bytes: u64,
    /// Disk usage percentage.
    pub disk_usage_percent: f64,
    /// Network bytes sent.
    pub network_bytes_sent: u64,
    /// Network bytes received.
    pub network_bytes_received: u64,
    /// Load average (1, 5, 15 min).
    pub load_average: [f64; 3],
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

#[cfg(test)]
impl Default for MockSystemResourcesWithUsage {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 45.2,
            memory_usage_percent: 62.8,
            available_memory_bytes: 4_000_000_000,
            total_memory_bytes: 8_000_000_000,
            disk_usage_percent: 25.0,
            network_bytes_sent: 1_000_000,
            network_bytes_received: 2_000_000,
            load_average: [0.5, 0.7, 0.9],
            uptime_seconds: 86400,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::ResourceMonitor;

    #[test]
    fn mock_resource_monitor_new() {
        let monitor = MockResourceMonitor::new();
        assert_eq!(
            std::mem::size_of_val(&monitor),
            std::mem::size_of::<MockResourceMonitor>()
        );
    }

    #[test]
    fn mock_resource_monitor_default() {
        let monitor = MockResourceMonitor::new();
        assert_eq!(
            std::mem::size_of_val(&monitor),
            std::mem::size_of::<MockResourceMonitor>()
        );
    }

    #[test]
    fn mock_resource_monitor_start_monitoring() {
        let monitor = MockResourceMonitor::new();
        let result = monitor.start_monitoring("workload-1");
        assert!(result.is_ok());
    }

    #[test]
    fn mock_resource_monitor_stop_monitoring() {
        let monitor = MockResourceMonitor::new();
        let result = monitor.stop_monitoring("workload-1");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_resource_monitor_get_metrics() {
        let monitor = MockResourceMonitor::new();
        let result = monitor.get_metrics("workload-1").await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(
            std::mem::size_of_val(&metrics),
            std::mem::size_of::<toadstool::RuntimeMetrics>()
        );
    }

    #[tokio::test]
    async fn mock_resource_monitor_get_system_resources() {
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

    #[test]
    fn mock_system_resources_with_usage_default() {
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
    fn mock_system_resources_with_usage_custom() {
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
        assert_eq!(resources.memory_usage_percent, 90.0);
        assert_eq!(resources.uptime_seconds, 172800);
        assert_eq!(resources.load_average[0], 1.0);
    }
}
