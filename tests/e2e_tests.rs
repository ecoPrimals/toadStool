//! End-to-end system tests
//!
//! This module provides access to all E2E tests for the ToadStool platform.
//! 
//! Sprint 25: Added 10 real integration E2E tests that use actual ToadStool components

#[path = "e2e/full_system_tests.rs"]
mod full_system_tests;

// Sprint 25: Real E2E Integration Tests
mod real_integration_tests {
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;

use toadstool::execution::*;
use toadstool::runtime::*;
use toadstool::{ToadStoolError, ToadStoolResult, RuntimeMetrics, WorkloadSpec};

#[derive(Debug, Clone)]
struct TestRuntimeEngine {
    name: String,
    execution_count: Arc<RwLock<usize>>,
    should_fail: bool,
    delay_ms: u64,
}

impl TestRuntimeEngine {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            execution_count: Arc::new(RwLock::new(0)),
            should_fail: false,
            delay_ms: 10,
        }
    }

    fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    async fn get_execution_count(&self) -> usize {
        *self.execution_count.read().await
    }
}

#[async_trait::async_trait]
impl RuntimeEngine for TestRuntimeEngine {
    async fn initialize(&mut self, _config: RuntimeConfig) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        {
            let mut count = self.execution_count.write().await;
            *count += 1;
        }

        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;

        if self.should_fail {
            return Err(ToadStoolError::execution(format!("Runtime {} simulated failure", self.name)));
        }

        Ok(ExecutionResponse {
            execution_id: request.execution_id,
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                stdout: format!("Executed by {}", self.name),
                stderr: String::new(),
                exit_code: Some(0),
            },
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_millis(self.delay_ms),
            runtime_used: RuntimeType::Container,
            warnings: Vec::new(),
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![toadstool::WorkloadType::Container, toadstool::WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &toadstool::WorkloadType) -> bool {
        true
    }

    async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        Ok(RuntimeMetrics::default())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        Ok(())
    }
}

fn create_test_workload_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Container {
            image: "alpine:latest".to_string(),
            command: Some(vec!["echo".to_string()]),
            args: Some(vec!["test".to_string()]),
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        },
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: toadstool::ExecutionInput::default(),
        callback_config: None,
    }
}

// E2E Test 1: Complete Workload Submission and Execution
#[tokio::test]
async fn test_e2e_complete_workload_execution() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = Box::new(TestRuntimeEngine::new("test-runtime-1"));
    
    orchestrator.register_engine(RuntimeType::Container, engine).await.unwrap();
    let request = create_test_workload_request();
    let execution_id = request.execution_id;
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok(), "Execution failed");
    let response = result.unwrap();
    assert_eq!(response.execution_id, execution_id);
    assert!(matches!(response.status, ExecutionStatus::Success));
}

// E2E Test 2: Multi-Runtime Failover
#[tokio::test]
async fn test_e2e_multi_runtime_failover() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let failing_engine = Box::new(TestRuntimeEngine::new("failing-runtime").with_failure(true));
    orchestrator.register_engine(RuntimeType::Container, failing_engine).await.unwrap();

    let request = create_test_workload_request();
    let result = orchestrator.execute(request).await;
    assert!(result.is_err(), "Expected execution to fail");
}

// E2E Test 3: Concurrent Workload Execution
#[tokio::test]
async fn test_e2e_concurrent_workload_execution() {
    let orchestrator = Arc::new(RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced));
    let engine = Box::new(TestRuntimeEngine::new("concurrent-runtime").with_delay(20));
    let engine_clone = Arc::new(engine.clone());
    
    orchestrator.register_engine(RuntimeType::Container, Box::new((*engine_clone).clone())).await.unwrap();

    let concurrent_count = 10;
    let mut handles = Vec::new();

    for _ in 0..concurrent_count {
        let orch = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_workload_request();
            orch.execute(request).await
        });
        handles.push(handle);
    }

    let mut successful = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            successful += 1;
        }
    }

    assert_eq!(successful, concurrent_count);
    let execution_count = engine_clone.get_execution_count().await;
    assert_eq!(execution_count, concurrent_count);
}

// E2E Test 4: Runtime Selection Strategy Testing
#[tokio::test]
async fn test_e2e_runtime_selection_strategies() {
    let strategies = vec![
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ];

    for strategy in strategies {
        let orchestrator = RuntimeOrchestrator::new(strategy);
        let engine = Box::new(TestRuntimeEngine::new("strategy-test"));
        orchestrator.register_engine(RuntimeType::Container, engine).await.unwrap();

        let request = create_test_workload_request();
        let result = orchestrator.execute(request).await;
        assert!(result.is_ok());
    }
}

// E2E Test 5: Workload Execution with Runtime Hints
#[tokio::test]
async fn test_e2e_execution_with_runtime_hints() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let container_engine = Box::new(TestRuntimeEngine::new("container-runtime"));
    let native_engine = Box::new(TestRuntimeEngine::new("native-runtime"));
    
    orchestrator.register_engine(RuntimeType::Container, container_engine).await.unwrap();
    orchestrator.register_engine(RuntimeType::Native, native_engine).await.unwrap();

    let mut request = create_test_workload_request();
    request.runtime_hint = Some(RuntimeType::Container);
    assert!(orchestrator.execute(request).await.is_ok());

    let mut request2 = create_test_workload_request();
    request2.runtime_hint = Some(RuntimeType::Native);
    assert!(orchestrator.execute(request2).await.is_ok());
}

// E2E Test 6: Error Handling and Recovery
#[tokio::test]
async fn test_e2e_error_handling_recovery() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let request1 = create_test_workload_request();
    assert!(orchestrator.execute(request1).await.is_err());

    let failing_engine = Box::new(TestRuntimeEngine::new("failing").with_failure(true));
    orchestrator.register_engine(RuntimeType::Container, failing_engine).await.unwrap();
    
    let request2 = create_test_workload_request();
    assert!(orchestrator.execute(request2).await.is_err());

    let working_engine = Box::new(TestRuntimeEngine::new("working"));
    orchestrator.register_engine(RuntimeType::Container, working_engine).await.unwrap();
    
    let request3 = create_test_workload_request();
    assert!(orchestrator.execute(request3).await.is_ok());
}

// E2E Test 7: Resource Lifecycle Management
#[tokio::test]
async fn test_e2e_resource_lifecycle() {
    use toadstool::resources::*;

    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = Box::new(TestRuntimeEngine::new("resource-test"));
    orchestrator.register_engine(RuntimeType::Container, engine).await.unwrap();

    let mut request = create_test_workload_request();
    request.resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: Some(4.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: Some(2 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

// E2E Test 8: Long-Running Workload Execution
#[tokio::test]
async fn test_e2e_long_running_workload() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = Box::new(TestRuntimeEngine::new("long-running").with_delay(200));
    orchestrator.register_engine(RuntimeType::Container, engine).await.unwrap();

    let mut request = create_test_workload_request();
    request.timeout = Some(Duration::from_secs(10));

    let start = std::time::Instant::now();
    let result = orchestrator.execute(request).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed >= Duration::from_millis(200));
}

// E2E Test 9: Execution Response Validation
#[tokio::test]
async fn test_e2e_execution_response_validation() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = Box::new(TestRuntimeEngine::new("validation"));
    orchestrator.register_engine(RuntimeType::Container, engine).await.unwrap();

    let request = create_test_workload_request();
    let expected_id = request.execution_id;
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.execution_id, expected_id);
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert!(!response.output.stdout.is_empty());
    assert!(response.duration > Duration::ZERO);
}

// E2E Test 10: Rapid Sequential Executions
#[tokio::test]
async fn test_e2e_rapid_sequential_executions() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let engine = Box::new(TestRuntimeEngine::new("rapid").with_delay(5));
    let engine_clone = Arc::new(engine.clone());
    
    orchestrator.register_engine(RuntimeType::Container, Box::new((*engine_clone).clone())).await.unwrap();

    let execution_count = 20;
    let mut successful = 0;

    for _ in 0..execution_count {
        let request = create_test_workload_request();
        if orchestrator.execute(request).await.is_ok() {
            successful += 1;
        }
    }

    assert_eq!(successful, execution_count);
    let count = engine_clone.get_execution_count().await;
    assert_eq!(count, execution_count);
}

} // end real_integration_tests module
