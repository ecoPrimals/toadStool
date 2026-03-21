// SPDX-License-Identifier: AGPL-3.0-only
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Mock resource monitors for testing

use mockall::mock;
use std::future::Future;
use std::pin::Pin;

use toadstool::{
    error::ToadStoolResult,
    resources::{ResourceMonitor, RuntimeMetrics, SystemResources},
};

use crate::fixtures::create_test_runtime_metrics;

// Mock trait for ResourceMonitor
mock! {
    pub ResourceMonitor {}

    impl std::fmt::Debug for ResourceMonitor {
        fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result;
    }

    impl ResourceMonitor for ResourceMonitor {
        fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;
        fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()>;
        fn get_metrics(&self, workload_id: &str) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send>>;
        fn get_system_resources(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send>>;
    }
}

impl MockResourceMonitor {
    /// Create a mock resource monitor that works successfully
    #[must_use]
    pub fn new_successful() -> Self {
        let mut mock = Self::new();

        // Debug implementation
        mock.expect_fmt()
            .returning(|f| write!(f, "MockResourceMonitor"));

        mock.expect_start_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_stop_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_get_metrics()
            .returning(|_workload_id| Box::pin(async move { Ok(create_test_runtime_metrics()) }));

        mock.expect_get_system_resources().returning(|| {
            Box::pin(async move {
                Ok(SystemResources {
                    available_cpu_cores: 8.0,
                    available_memory_bytes: 16 * 1024 * 1024 * 1024, // 16GB
                    available_storage_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
                    available_network_bandwidth: Some(1000000000),   // 1Gbps
                    available_gpu_units: 1,
                    cpu_usage_percent: 25.0,
                    memory_usage_percent: 50.0,
                    total_cpu_cores: 16,
                    total_memory_bytes: 32 * 1024 * 1024 * 1024, // 32GB
                })
            })
        });

        mock
    }

    /// Create a mock resource monitor that reports limit violations
    #[must_use]
    pub fn new_limit_violations() -> Self {
        let mut mock = Self::new();

        mock.expect_fmt()
            .returning(|f| write!(f, "MockResourceMonitor(LimitViolations)"));

        mock.expect_start_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_stop_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_get_metrics().returning(|_workload_id| {
            Box::pin(async move {
                let mut metrics = create_test_runtime_metrics();
                metrics.cpu.usage_percent = 95.0; // High CPU usage
                metrics.memory.used_bytes = 1024 * 1024 * 1024 * 7; // 7GB memory usage
                Ok(metrics)
            })
        });

        mock.expect_get_system_resources().returning(|| {
            Box::pin(async move {
                Ok(SystemResources {
                    available_cpu_cores: 2.0,                          // Limited resources
                    available_memory_bytes: 4 * 1024 * 1024 * 1024,    // 4GB
                    available_storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
                    available_network_bandwidth: Some(100000000),      // 100Mbps
                    available_gpu_units: 0,
                    cpu_usage_percent: 75.0,    // High usage
                    memory_usage_percent: 87.5, // 7GB/8GB = 87.5%
                    total_cpu_cores: 8,
                    total_memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB
                })
            })
        });

        mock
    }

    /// Create a mock resource monitor that fails operations
    #[must_use]
    pub fn new_monitoring_failure() -> Self {
        let mut mock = Self::new();

        mock.expect_fmt()
            .returning(|f| write!(f, "MockResourceMonitor(Failure)"));

        mock.expect_start_monitoring().returning(|_workload_id| {
            Err(toadstool::error::ToadStoolError::resource(
                "Failed to start monitoring",
            ))
        });

        mock.expect_stop_monitoring().returning(|_workload_id| {
            Err(toadstool::error::ToadStoolError::resource(
                "Failed to stop monitoring",
            ))
        });

        mock.expect_get_metrics().returning(|_workload_id| {
            Box::pin(async move {
                Err(toadstool::error::ToadStoolError::resource(
                    "Failed to get metrics",
                ))
            })
        });

        mock.expect_get_system_resources().returning(|| {
            Box::pin(async move {
                Err(toadstool::error::ToadStoolError::resource(
                    "Failed to get system resources",
                ))
            })
        });

        mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_mock() {
        let monitor = MockResourceMonitor::new_successful();

        // Test monitoring lifecycle
        assert!(monitor.start_monitoring("test-workload").is_ok());
        assert!(monitor.stop_monitoring("test-workload").is_ok());

        // Test metrics (now async!)
        let metrics = monitor.get_metrics("test-workload").await.unwrap();
        assert!(metrics.cpu.usage_percent >= 0.0);

        // Test system resources
        let system_resources = monitor.get_system_resources().await.unwrap();
        assert!(system_resources.available_cpu_cores > 0.0);
    }

    #[tokio::test]
    async fn test_limit_violations_mock() {
        let monitor = MockResourceMonitor::new_limit_violations();

        assert!(monitor.start_monitoring("test-workload").is_ok());

        let metrics = monitor.get_metrics("test-workload").await.unwrap();
        assert!(metrics.cpu.usage_percent > 90.0); // High CPU usage

        // Test that limited resources are reported
        let system_resources = monitor.get_system_resources().await.unwrap();
        assert!(system_resources.available_cpu_cores < 4.0); // Limited resources
        assert!(system_resources.available_memory_bytes < 8 * 1024 * 1024 * 1024); // Less than 8GB

        assert!(monitor.stop_monitoring("test-workload").is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_failure_mock() {
        let monitor = MockResourceMonitor::new_monitoring_failure();

        // Should fail to start monitoring
        assert!(monitor.start_monitoring("test-workload").is_err());

        // Should fail to get metrics
        assert!(monitor.get_metrics("test-workload").await.is_err());

        // Should fail to stop monitoring
        assert!(monitor.stop_monitoring("test-workload").is_err());

        // Should fail to get system resources
        assert!(monitor.get_system_resources().await.is_err());
    }
}
