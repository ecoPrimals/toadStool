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

use toadstool::{
    error::ToadStoolResult,
    resources::{ResourceMonitor, ResourceRequirements, RuntimeMetrics},
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
        fn get_metrics(&self, workload_id: &str) -> ToadStoolResult<RuntimeMetrics>;
        fn check_limits(&self, workload_id: &str, requirements: &ResourceRequirements) -> ToadStoolResult<bool>;
    }
}

impl MockResourceMonitor {
    /// Create a mock resource monitor that works successfully
    pub fn new_successful() -> Self {
        let mut mock = MockResourceMonitor::new();

        // Debug implementation
        mock.expect_fmt()
            .returning(|f| write!(f, "MockResourceMonitor"));

        mock.expect_start_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_stop_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_get_metrics()
            .returning(|_workload_id| Ok(create_test_runtime_metrics()));

        mock.expect_check_limits()
            .returning(|_workload_id, _requirements| Ok(true));

        mock
    }

    /// Create a mock resource monitor that reports limit violations
    pub fn new_limit_violations() -> Self {
        let mut mock = MockResourceMonitor::new();

        mock.expect_fmt()
            .returning(|f| write!(f, "MockResourceMonitor(LimitViolations)"));

        mock.expect_start_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_stop_monitoring()
            .returning(|_workload_id| Ok(()));

        mock.expect_get_metrics().returning(|_workload_id| {
            let mut metrics = create_test_runtime_metrics();
            metrics.cpu.usage_percent = 95.0; // High CPU usage
            metrics.memory.usage_bytes = 1024 * 1024 * 1024 * 7; // 7GB memory usage
            Ok(metrics)
        });

        mock.expect_check_limits()
            .returning(|_workload_id, requirements| {
                // Simulate limit check failure for high resource requests
                let cpu_ok = requirements.cpu.min_cores < 1.0;
                let memory_ok = requirements.memory.min_bytes < 1024 * 1024 * 1024; // 1GB
                Ok(cpu_ok && memory_ok)
            });

        mock
    }

    /// Create a mock resource monitor that fails operations
    pub fn new_monitoring_failure() -> Self {
        let mut mock = MockResourceMonitor::new();

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
            Err(toadstool::error::ToadStoolError::resource(
                "Failed to get metrics",
            ))
        });

        mock.expect_check_limits()
            .returning(|_workload_id, _requirements| {
                Err(toadstool::error::ToadStoolError::resource(
                    "Failed to check limits",
                ))
            });

        mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::create_test_resource_requirements;

    #[test]
    fn test_successful_mock() {
        let monitor = MockResourceMonitor::new_successful();

        // Test monitoring lifecycle
        assert!(monitor.start_monitoring("test-workload").is_ok());
        assert!(monitor.stop_monitoring("test-workload").is_ok());

        // Test metrics
        let metrics = monitor.get_metrics("test-workload").unwrap();
        assert!(metrics.cpu.usage_percent >= 0.0);

        // Test limit checking
        let requirements = create_test_resource_requirements();
        assert!(monitor
            .check_limits("test-workload", &requirements)
            .unwrap());
    }

    #[test]
    fn test_limit_violations_mock() {
        let monitor = MockResourceMonitor::new_limit_violations();

        assert!(monitor.start_monitoring("test-workload").is_ok());

        let metrics = monitor.get_metrics("test-workload").unwrap();
        assert!(metrics.cpu.usage_percent > 90.0); // High CPU usage

        // Test that large requests are rejected
        let mut large_requirements = create_test_resource_requirements();
        large_requirements.cpu.min_cores = 8.0;
        large_requirements.memory.min_bytes = 1024 * 1024 * 1024 * 16; // 16GB

        let result = monitor
            .check_limits("test-workload", &large_requirements)
            .unwrap();
        assert!(!result); // Should be rejected

        assert!(monitor.stop_monitoring("test-workload").is_ok());
    }

    #[test]
    fn test_monitoring_failure_mock() {
        let monitor = MockResourceMonitor::new_monitoring_failure();

        // Should fail to start monitoring
        assert!(monitor.start_monitoring("test-workload").is_err());

        // Should fail to get metrics
        assert!(monitor.get_metrics("test-workload").is_err());

        // Should fail to stop monitoring
        assert!(monitor.stop_monitoring("test-workload").is_err());

        // Should fail to check limits
        let requirements = create_test_resource_requirements();
        assert!(monitor
            .check_limits("test-workload", &requirements)
            .is_err());
    }
}
