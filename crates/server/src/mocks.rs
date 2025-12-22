//! Mock implementations for testing
//!
//! ⚠️ **TEST-ONLY MODULE**
//! These mocks are for testing infrastructure only and should never be used in production.

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
            })
        })
    }
}

/// Mock system resources for testing (with usage metrics)
#[cfg(test)]
pub struct MockSystemResourcesWithUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub available_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub disk_usage_percent: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub load_average: [f64; 3],
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
