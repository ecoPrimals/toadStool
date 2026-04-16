// SPDX-License-Identifier: AGPL-3.0-or-later
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

use std::future::Future;

use toadstool::{
    ResourceMonitor, ResourceMonitorDispatch, RuntimeMetrics, SystemResources, TestResourceMonitor,
    ToadStoolResult,
};

/// Mock resource monitor (delegates to [`ResourceMonitorDispatch::Test`] presets).
#[derive(Clone, Debug)]
pub struct MockResourceMonitor {
    inner: ResourceMonitorDispatch,
}

impl MockResourceMonitor {
    /// Create a mock resource monitor that works successfully
    #[must_use]
    pub fn new_successful() -> Self {
        Self {
            inner: ResourceMonitorDispatch::Test(TestResourceMonitor::successful()),
        }
    }

    /// Create a mock resource monitor that reports limit violations
    #[must_use]
    pub fn new_limit_violations() -> Self {
        Self {
            inner: ResourceMonitorDispatch::Test(
                toadstool::resources::TestResourceMonitor::limit_violations(),
            ),
        }
    }

    /// Create a mock resource monitor that fails operations
    #[must_use]
    pub fn new_monitoring_failure() -> Self {
        Self {
            inner: ResourceMonitorDispatch::Test(
                toadstool::resources::TestResourceMonitor::monitoring_failure(),
            ),
        }
    }

    /// Returns the underlying [`ResourceMonitorDispatch`] for `Arc<…>`-style APIs.
    #[must_use]
    pub fn into_dispatch(self) -> ResourceMonitorDispatch {
        self.inner
    }
}

impl ResourceMonitor for MockResourceMonitor {
    fn start_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        self.inner.start_monitoring(workload_id)
    }

    fn stop_monitoring(&self, workload_id: &str) -> ToadStoolResult<()> {
        self.inner.stop_monitoring(workload_id)
    }

    fn get_metrics(
        &self,
        workload_id: &str,
    ) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        let inner = self.inner.clone();
        let workload_id = workload_id.to_string();
        async move { inner.get_metrics(&workload_id).await }
    }

    fn get_system_resources(
        &self,
    ) -> impl Future<Output = ToadStoolResult<SystemResources>> + Send + '_ {
        let inner = self.inner.clone();
        async move { inner.get_system_resources().await }
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
