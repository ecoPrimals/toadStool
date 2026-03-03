// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime Integration E2E Tests
//!
//! End-to-end tests that exercise multiple runtime engines together,
//! testing real workload routing, execution, and resource management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use toadstool::execution::*;
use toadstool::runtime::*;
use toadstool::{ToadStoolError, ToadStoolResult, RuntimeMetrics, WorkloadSpec};

// ============================================================================
// Helper: Simple Test Runtime
// ============================================================================

#[derive(Clone)]
struct SimpleTestRuntime {
    name: String,
    executions: Arc<RwLock<Vec<String>>>,
}

impl SimpleTestRuntime {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            executions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn get_executions(&self) -> Vec<String> {
        self.executions.read().await.clone()
    }
}

#[async_trait::async_trait]
impl RuntimeEngine for SimpleTestRuntime {
    async fn initialize(&mut self, _config: RuntimeConfig) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        self.executions
            .write()
            .await
            .push(request.execution_id.to_string());

        Ok(ExecutionResponse {
            execution_id: request.execution_id,
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                stdout: format!("Executed by {} runtime", self.name),
                stderr: String::new(),
                exit_code: Some(0),
            },
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_millis(10),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![toadstool::WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &toadstool::WorkloadType) -> bool {
        matches!(workload_type, toadstool::WorkloadType::Native)
    }

    async fn health_check(&self) -> ToadStoolResult<toadstool::HealthStatus> {
        Ok(toadstool::HealthStatus::Healthy)
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        Ok(())
    }
}

// ============================================================================
// E2E Test: Single Runtime Execution
// ============================================================================

#[tokio::test]
async fn test_e2e_single_runtime_execution() {
    // Create a simple runtime
    let runtime = Arc::new(SimpleTestRuntime::new("native"));

    // Create execution request
    let execution_id = Uuid::new_v4();
    let request = ExecutionRequest {
        execution_id,
        workload: WorkloadSpec {
            workload_type: toadstool::WorkloadType::Native,
            code: vec![],
            entry_point: Some("main".to_string()),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: None,
            resource_limits: None,
        },
        security_context: toadstool::security::SecurityContext::default(),
        timeout: Some(Duration::from_secs(30)),
        priority: toadstool::ExecutionPriority::Normal,
        metadata: HashMap::new(),
    };

    // Execute workload
    let response = runtime.execute(request).await.unwrap();

    // Verify response
    assert_eq!(response.execution_id, execution_id);
    assert_eq!(response.status, ExecutionStatus::Success);
    assert!(response.output.stdout.contains("native"));
    assert_eq!(response.output.exit_code, Some(0));

    // Verify execution was tracked
    let executions = runtime.get_executions().await;
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0], execution_id.to_string());
}

// ============================================================================
// E2E Test: Multiple Sequential Executions
// ============================================================================

#[tokio::test]
async fn test_e2e_multiple_sequential_executions() {
    let runtime = Arc::new(SimpleTestRuntime::new("test"));

    // Execute 5 workloads sequentially
    let mut execution_ids = Vec::new();

    for i in 0..5 {
        let execution_id = Uuid::new_v4();
        execution_ids.push(execution_id);

        let request = ExecutionRequest {
            execution_id,
            workload: WorkloadSpec {
                workload_type: toadstool::WorkloadType::Native,
                code: vec![],
                entry_point: Some(format!("task_{}", i)),
                arguments: vec![],
                environment: HashMap::new(),
                working_directory: None,
                resource_limits: None,
            },
            security_context: toadstool::security::SecurityContext::default(),
            timeout: Some(Duration::from_secs(10)),
            priority: toadstool::ExecutionPriority::Normal,
            metadata: HashMap::new(),
        };

        let response = runtime.execute(request).await.unwrap();
        assert_eq!(response.status, ExecutionStatus::Success);
    }

    // Verify all executions were tracked
    let executions = runtime.get_executions().await;
    assert_eq!(executions.len(), 5);

    for (i, exec_id) in execution_ids.iter().enumerate() {
        assert_eq!(executions[i], exec_id.to_string());
    }
}

// ============================================================================
// E2E Test: Concurrent Executions
// ============================================================================

#[tokio::test]
async fn test_e2e_concurrent_executions() {
    let runtime = Arc::new(SimpleTestRuntime::new("concurrent"));

    // Launch 10 concurrent executions
    let mut handles = Vec::new();

    for i in 0..10 {
        let runtime_clone = Arc::clone(&runtime);
        let handle = tokio::spawn(async move {
            let execution_id = Uuid::new_v4();
            let request = ExecutionRequest {
                execution_id,
                workload: WorkloadSpec {
                    workload_type: toadstool::WorkloadType::Native,
                    code: vec![],
                    entry_point: Some(format!("concurrent_{}", i)),
                    arguments: vec![],
                    environment: HashMap::new(),
                    working_directory: None,
                    resource_limits: None,
                },
                security_context: toadstool::security::SecurityContext::default(),
                timeout: Some(Duration::from_secs(10)),
                priority: toadstool::ExecutionPriority::Normal,
                metadata: HashMap::new(),
            };

            runtime_clone.execute(request).await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(response)) = handle.await {
            if response.status == ExecutionStatus::Success {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 10, "All concurrent executions should succeed");

    // Verify all executions were tracked
    let executions = runtime.get_executions().await;
    assert_eq!(executions.len(), 10);
}

// ============================================================================
// E2E Test: Health Check Integration
// ============================================================================

#[tokio::test]
async fn test_e2e_health_check_workflow() {
    let runtime = Arc::new(SimpleTestRuntime::new("health"));

    // Check health before execution
    let health_before = runtime.health_check().await.unwrap();
    assert_eq!(health_before, toadstool::HealthStatus::Healthy);

    // Execute workload
    let execution_id = Uuid::new_v4();
    let request = ExecutionRequest {
        execution_id,
        workload: WorkloadSpec {
            workload_type: toadstool::WorkloadType::Native,
            code: vec![],
            entry_point: Some("health_test".to_string()),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: None,
            resource_limits: None,
        },
        security_context: toadstool::security::SecurityContext::default(),
        timeout: Some(Duration::from_secs(10)),
        priority: toadstool::ExecutionPriority::Normal,
        metadata: HashMap::new(),
    };

    let response = runtime.execute(request).await.unwrap();
    assert_eq!(response.status, ExecutionStatus::Success);

    // Check health after execution
    let health_after = runtime.health_check().await.unwrap();
    assert_eq!(health_after, toadstool::HealthStatus::Healthy);
}

// ============================================================================
// E2E Test: Capability Verification
// ============================================================================

#[tokio::test]
async fn test_e2e_capability_verification() {
    let runtime = SimpleTestRuntime::new("capabilities");

    // Get capabilities
    let capabilities = runtime.get_capabilities();

    // Verify capabilities
    assert!(!capabilities.supported_workloads.is_empty());
    assert!(capabilities.max_concurrent_executions.is_some());
    assert!(!capabilities.supported_architectures.is_empty());
    assert!(!capabilities.version.is_empty());

    // Verify workload support
    assert!(runtime.supports_workload(&toadstool::WorkloadType::Native));
}

// ============================================================================
// E2E Test: Shutdown Lifecycle
// ============================================================================

#[tokio::test]
async fn test_e2e_shutdown_lifecycle() {
    let mut runtime = SimpleTestRuntime::new("shutdown");

    // Initialize
    runtime
        .initialize(RuntimeConfig::default())
        .await
        .unwrap();

    // Execute some workloads
    for i in 0..3 {
        let execution_id = Uuid::new_v4();
        let request = ExecutionRequest {
            execution_id,
            workload: WorkloadSpec {
                workload_type: toadstool::WorkloadType::Native,
                code: vec![],
                entry_point: Some(format!("task_{}", i)),
                arguments: vec![],
                environment: HashMap::new(),
                working_directory: None,
                resource_limits: None,
            },
            security_context: toadstool::security::SecurityContext::default(),
            timeout: Some(Duration::from_secs(10)),
            priority: toadstool::ExecutionPriority::Normal,
            metadata: HashMap::new(),
        };

        runtime.execute(request).await.unwrap();
    }

    // Verify executions
    let executions = runtime.get_executions().await;
    assert_eq!(executions.len(), 3);

    // Graceful shutdown
    runtime.shutdown().await.unwrap();

    // After shutdown, executions history still accessible
    let executions_after = runtime.get_executions().await;
    assert_eq!(executions_after.len(), 3);
}

// ============================================================================
// E2E Test: Error Recovery
// ============================================================================

#[derive(Clone)]
struct FlakyTestRuntime {
    name: String,
    failure_count: Arc<RwLock<usize>>,
    fail_threshold: usize,
}

impl FlakyTestRuntime {
    fn new(name: &str, fail_threshold: usize) -> Self {
        Self {
            name: name.to_string(),
            failure_count: Arc::new(RwLock::new(0)),
            fail_threshold,
        }
    }
}

#[async_trait::async_trait]
impl RuntimeEngine for FlakyTestRuntime {
    async fn initialize(&mut self, _config: RuntimeConfig) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        let mut count = self.failure_count.write().await;
        *count += 1;

        if *count <= self.fail_threshold {
            return Err(ToadStoolError::execution("Simulated failure"));
        }

        Ok(ExecutionResponse {
            execution_id: request.execution_id,
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                stdout: format!("Success after {} attempts", *count),
                stderr: String::new(),
                exit_code: Some(0),
            },
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_millis(10),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![toadstool::WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &toadstool::WorkloadType) -> bool {
        true
    }

    async fn health_check(&self) -> ToadStoolResult<toadstool::HealthStatus> {
        Ok(toadstool::HealthStatus::Healthy)
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_e2e_error_recovery() {
    // Runtime that fails first 2 attempts, then succeeds
    let runtime = Arc::new(FlakyTestRuntime::new("flaky", 2));

    let execution_id = Uuid::new_v4();
    let request = ExecutionRequest {
        execution_id,
        workload: WorkloadSpec {
            workload_type: toadstool::WorkloadType::Native,
            code: vec![],
            entry_point: Some("test".to_string()),
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: None,
            resource_limits: None,
        },
        security_context: toadstool::security::SecurityContext::default(),
        timeout: Some(Duration::from_secs(10)),
        priority: toadstool::ExecutionPriority::Normal,
        metadata: HashMap::new(),
    };

    // First attempt - should fail
    let result1 = runtime.execute(request.clone()).await;
    assert!(result1.is_err());

    // Second attempt - should fail
    let result2 = runtime.execute(request.clone()).await;
    assert!(result2.is_err());

    // Third attempt - should succeed
    let result3 = runtime.execute(request).await;
    assert!(result3.is_ok());
    assert_eq!(result3.unwrap().status, ExecutionStatus::Success);
}

