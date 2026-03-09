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

//! Mock runtime engines for testing
//!
//! This module provides mock implementations of runtime engines using the
//! mockall crate for comprehensive testing scenarios.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use mockall::mock;

use toadstool::{
    error::ToadStoolResult,
    execution::{
        ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
        RuntimeEngine, RuntimeType,
    },
    resources::RuntimeMetrics,
    WorkloadType,
};

use crate::fixtures::{create_test_execution_output, create_test_runtime_metrics};

// Mock trait for RuntimeEngine
mock! {
    pub RuntimeEngine {}

    impl RuntimeEngine for RuntimeEngine {
        fn initialize(&mut self, config: RuntimeConfig) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send>>;
        fn execute(&self, request: ExecutionRequest) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send>>;
        fn get_capabilities(&self) -> RuntimeCapabilities;
        fn supports_workload(&self, workload_type: &WorkloadType) -> bool;
        fn get_metrics(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send>>;
        fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send>>;
    }
}

// Manual Debug implementation for MockRuntimeEngine
impl std::fmt::Debug for MockRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockRuntimeEngine").finish()
    }
}

impl MockRuntimeEngine {
    /// Create a mock runtime engine that always succeeds
    #[must_use]
    pub fn new_successful() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Success,
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(5),
                    runtime_used: RuntimeType::Native,
                    warnings: vec![],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![
                    WorkloadType::Native,
                    WorkloadType::Container,
                    WorkloadType::Wasm,
                ],
                max_concurrent_executions: Some(10),
                supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test".to_string(),
            });

        mock.expect_supports_workload().returning(|workload_type| {
            matches!(
                workload_type,
                WorkloadType::Native | WorkloadType::Container | WorkloadType::Wasm
            )
        });

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine that always fails initialization
    #[must_use]
    pub fn new_init_failure() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize().returning(|_| {
            Box::pin(async {
                Err(toadstool::error::ToadStoolError::runtime(
                    "Initialization failed",
                ))
            })
        });

        mock
    }

    /// Create a mock runtime engine that fails execution
    #[must_use]
    pub fn new_execution_failure() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Failed {
                        error: std::borrow::Cow::Borrowed("Mock execution failure"),
                    },
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(1),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Mock warning".to_string()],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-fail".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine that fails get_metrics (for health check testing)
    #[must_use]
    pub fn new_metrics_failure() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Success,
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(1),
                    runtime_used: RuntimeType::Native,
                    warnings: vec![],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-metrics-fail".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics().returning(|| {
            Box::pin(async move {
                Err(toadstool::error::ToadStoolError::resource(
                    "Health check: failed to get engine metrics",
                ))
            })
        });

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine that times out
    #[must_use]
    pub fn new_timeout() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::TimedOut,
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(30),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Execution timed out".to_string()],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-timeout".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine with resource limit exceeded
    #[must_use]
    pub fn new_resource_limit_exceeded() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Failed {
                        error: std::borrow::Cow::Borrowed(
                            "Resource limit exceeded: memory limit 1GB, actual 2GB",
                        ),
                    },
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(2),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Memory limit exceeded".to_string()],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-limit".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine with security violation
    #[must_use]
    pub fn new_security_violation() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Failed {
                        error: std::borrow::Cow::Borrowed(
                            "Security violation: Attempted to access restricted file",
                        ),
                    },
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_millis(100),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Security policy violation detected".to_string()],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-security".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine that was cancelled
    #[must_use]
    pub fn new_cancelled() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Cancelled,
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_millis(500),
                    runtime_used: RuntimeType::Native,
                    warnings: vec!["Execution was cancelled".to_string()],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Native],
                max_concurrent_executions: Some(1),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: std::collections::HashMap::new(),
                version: "1.0.0-test-cancel".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Native));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }

    /// Create a mock runtime engine with limited workload support
    #[must_use]
    pub fn new_limited_support() -> Self {
        let mut mock = MockRuntimeEngine::new();

        mock.expect_initialize()
            .returning(|_| Box::pin(async { Ok(()) }));

        mock.expect_execute().returning(|request| {
            Box::pin(async move {
                Ok(ExecutionResponse {
                    execution_id: request.execution_id,
                    status: ExecutionStatus::Success,
                    output: create_test_execution_output(),
                    metrics: create_test_runtime_metrics(),
                    duration: Duration::from_secs(3),
                    runtime_used: RuntimeType::Wasm,
                    warnings: vec![],
                })
            })
        });

        mock.expect_get_capabilities()
            .returning(|| RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Wasm],
                max_concurrent_executions: Some(5),
                supported_architectures: vec!["x86_64".to_string()],
                platform_features: {
                    let mut features = std::collections::HashMap::new();
                    features.insert("wasi".to_string(), true);
                    features.insert("simd".to_string(), false);
                    features
                },
                version: "1.0.0-wasm-only".to_string(),
            });

        mock.expect_supports_workload()
            .returning(|workload_type| matches!(workload_type, WorkloadType::Wasm));

        mock.expect_get_metrics()
            .returning(|| Box::pin(async { Ok(create_test_runtime_metrics()) }));

        mock.expect_shutdown()
            .returning(|| Box::pin(async { Ok(()) }));

        mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::create_test_execution_request;

    #[tokio::test(flavor = "current_thread")]
    async fn test_successful_mock() {
        let mut mock = MockRuntimeEngine::new_successful();

        // Test initialization
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        // Test execution
        let request = create_test_execution_request();
        let response = mock
            .execute(request.clone())
            .await
            .expect("Mock execution should succeed");
        assert_eq!(response.execution_id, request.execution_id);
        assert_eq!(response.status, ExecutionStatus::Success);

        // Test capabilities
        let capabilities = mock.get_capabilities();
        assert!(!capabilities.supported_workloads.is_empty());
        assert!(mock.supports_workload(&WorkloadType::Native));

        // Test metrics
        assert!(mock.get_metrics().await.is_ok());

        // Test shutdown
        assert!(mock.shutdown().await.is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_failure_mocks() {
        // Test initialization failure
        let mut init_fail_mock = MockRuntimeEngine::new_init_failure();
        assert!(init_fail_mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_err());

        // Test execution failure
        let mut exec_fail_mock = MockRuntimeEngine::new_execution_failure();
        assert!(exec_fail_mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        let request = create_test_execution_request();
        let response = exec_fail_mock.execute(request).await.unwrap();
        assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_timeout_mock() {
        let mut mock = MockRuntimeEngine::new_timeout();
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        let request = create_test_execution_request();
        let response = mock.execute(request).await.unwrap();
        assert_eq!(response.status, ExecutionStatus::TimedOut);
        assert_eq!(response.duration.as_secs(), 30);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_resource_limit_mock() {
        let mut mock = MockRuntimeEngine::new_resource_limit_exceeded();
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        let request = create_test_execution_request();
        let response = mock.execute(request).await.unwrap();
        assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_security_violation_mock() {
        let mut mock = MockRuntimeEngine::new_security_violation();
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        let request = create_test_execution_request();
        let response = mock.execute(request).await.unwrap();
        assert!(matches!(response.status, ExecutionStatus::Failed { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_cancelled_mock() {
        let mut mock = MockRuntimeEngine::new_cancelled();
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        let request = create_test_execution_request();
        let response = mock.execute(request).await.unwrap();
        assert_eq!(response.status, ExecutionStatus::Cancelled);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_limited_support_mock() {
        let mut mock = MockRuntimeEngine::new_limited_support();
        assert!(mock
            .initialize(crate::fixtures::create_test_runtime_config())
            .await
            .is_ok());

        // Should support WASM
        assert!(mock.supports_workload(&WorkloadType::Wasm));

        // Should not support others
        assert!(!mock.supports_workload(&WorkloadType::Native));
        assert!(!mock.supports_workload(&WorkloadType::Container));

        let capabilities = mock.get_capabilities();
        assert_eq!(capabilities.supported_workloads, vec![WorkloadType::Wasm]);
        assert!(capabilities.platform_features.get("wasi").unwrap_or(&false));
    }
}
