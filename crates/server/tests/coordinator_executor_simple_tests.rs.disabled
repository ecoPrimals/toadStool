//! Simple integration tests for CoordinatorExecutor
//!
//! Tests basic functionality of the distributed coordinator integration.

use toadstool_distributed::DistributedConfig;
use toadstool_integration_protocols::tarpc_service::{ResourceRequirements, WorkloadSubmission};
use toadstool_server::tarpc_server::WorkloadExecutor;
use toadstool_server::CoordinatorExecutor;

/// Helper to create a test workload submission
fn create_test_submission(workload_type: &str) -> WorkloadSubmission {
    use toadstool_integration_protocols::tarpc_service::WorkloadPriority;

    WorkloadSubmission {
        workload_id: uuid::Uuid::new_v4().to_string(),
        workload_type: workload_type.to_string(),
        data: vec![1, 2, 3, 4, 5],
        requirements: ResourceRequirements {
            cpu_cores: Some(2),
            memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            gpu_memory_bytes: None,
            timeout_secs: Some(30),
        },
        metadata: std::collections::HashMap::new(),
        priority: WorkloadPriority::Normal,
    }
}

/// Helper to create a test config
fn create_test_config() -> DistributedConfig {
    use toadstool_distributed::core::StandaloneConfig;

    DistributedConfig {
        instance_id: "test-instance".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    }
}

#[tokio::test]
async fn test_coordinator_executor_creation() {
    let config = create_test_config();
    let result = CoordinatorExecutor::new(config, "test-service".to_string()).await;
    assert!(
        result.is_ok(),
        "Should create coordinator executor successfully"
    );
}

#[tokio::test]
async fn test_coordinator_executor_cpu_workload() {
    let config = create_test_config();
    let executor = CoordinatorExecutor::new(config, "test-service".to_string())
        .await
        .expect("Should create executor");

    let submission = create_test_submission("cpu_compute");
    let result = executor.execute(submission).await;

    assert!(result.is_ok(), "CPU workload should execute successfully");
}

#[tokio::test]
async fn test_coordinator_executor_query_capabilities() {
    let config = create_test_config();
    let executor = CoordinatorExecutor::new(config, "test-service".to_string())
        .await
        .expect("Should create executor");

    let result = executor.query_capabilities().await;
    assert!(result.is_ok(), "Should query capabilities successfully");
}

#[tokio::test]
async fn test_coordinator_executor_multiple_workloads() {
    let config = create_test_config();
    let executor = CoordinatorExecutor::new(config, "test-service".to_string())
        .await
        .expect("Should create executor");

    // Submit multiple workloads
    for _ in 0..3 {
        let submission = create_test_submission("cpu_compute");
        let result = executor.execute(submission).await;
        assert!(result.is_ok(), "Workload should execute");
    }
}

#[tokio::test]
async fn test_coordinator_executor_cancel() {
    let config = create_test_config();
    let executor = CoordinatorExecutor::new(config, "test-service".to_string())
        .await
        .expect("Should create executor");

    let submission = create_test_submission("cpu_compute");
    let workload_id = submission.workload_id.clone();

    // Execute workload
    let result = executor.execute(submission).await;
    assert!(result.is_ok(), "Workload should execute");

    // Cancel workload
    let cancel_result = executor.cancel(&workload_id).await;
    assert!(cancel_result.is_ok(), "Should be able to cancel workload");
}
