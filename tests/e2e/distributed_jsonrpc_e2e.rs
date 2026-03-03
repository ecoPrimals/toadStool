// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-End Tests for Distributed Coordinator + JSON-RPC Server Integration
//!
//! Tests the complete workflow from JSON-RPC client → Server → Coordinator → Execution
//! This validates the full stack integration for deep debt compliance.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use toadstool_server::{CoordinatorExecutor, StandaloneExecutor};
use toadstool_server::tarpc_server::WorkloadExecutor;
use toadstool_integration_protocols::tarpc_service::{
    ResourceRequirements, WorkloadPriority, WorkloadSubmission,
};
use toadstool_distributed::{DistributedConfig, DistributedCoordinator};
use toadstool_distributed::core::StandaloneConfig;

// ============================================================================
// Test Helpers
// ============================================================================

async fn create_test_coordinator() -> Result<Arc<DistributedCoordinator>, String> {
    let config = DistributedConfig {
        instance_id: format!("test-{}", Uuid::new_v4()),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config)
        .await
        .map_err(|e| format!("Failed to create coordinator: {}", e))?;
    
    Ok(Arc::new(coordinator))
}

async fn create_coordinator_executor() -> Result<CoordinatorExecutor, String> {
    let config = DistributedConfig {
        instance_id: format!("test-{}", Uuid::new_v4()),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 30,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        songbird_integration: None,
    };

    CoordinatorExecutor::new(config, "test-service".to_string()).await
}

fn create_test_workload(workload_type: &str) -> WorkloadSubmission {
    WorkloadSubmission {
        workload_id: Uuid::new_v4().to_string(),
        workload_type: workload_type.to_string(),
        data: vec![1, 2, 3, 4, 5],
        requirements: ResourceRequirements {
            cpu_cores: Some(2),
            memory_bytes: Some(1024 * 1024 * 1024),
            gpu_memory_bytes: None,
            timeout_secs: Some(30),
        },
        metadata: std::collections::HashMap::new(),
        priority: WorkloadPriority::Normal,
    }
}

// ============================================================================
// E2E: Coordinator Lifecycle
// ============================================================================

#[tokio::test]
async fn e2e_coordinator_initialization() {
    let result = create_test_coordinator().await;
    assert!(result.is_ok(), "Coordinator should initialize successfully");
}

#[tokio::test]
async fn e2e_coordinator_start_stop() {
    let coordinator = create_test_coordinator().await.expect("Create coordinator");
    
    // Start coordinator
    let start_result = Arc::clone(&coordinator).start().await;
    assert!(start_result.is_ok(), "Coordinator should start successfully");
    
    // Coordinator is now running (no direct is_running() method, but start succeeded)
}

#[tokio::test]
async fn e2e_coordinator_multiple_instances() {
    // Test that multiple coordinator instances can coexist
    let coord1 = create_test_coordinator().await.expect("Create coordinator 1");
    let coord2 = create_test_coordinator().await.expect("Create coordinator 2");
    
    let start1 = Arc::clone(&coord1).start().await;
    let start2 = Arc::clone(&coord2).start().await;
    
    assert!(start1.is_ok(), "Coordinator 1 should start");
    assert!(start2.is_ok(), "Coordinator 2 should start");
}

// ============================================================================
// E2E: Workload Execution via Coordinator
// ============================================================================

#[tokio::test]
async fn e2e_single_workload_execution() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("cpu_compute");
    
    let result = timeout(Duration::from_secs(5), executor.execute(workload)).await;
    
    assert!(result.is_ok(), "Execution should not timeout");
    assert!(result.unwrap().is_ok(), "Workload should execute successfully");
}

#[tokio::test]
async fn e2e_multiple_sequential_workloads() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    
    for i in 0..5 {
        let mut workload = create_test_workload("cpu_compute");
        workload.metadata.insert("index".to_string(), i.to_string());
        
        let result = timeout(Duration::from_secs(5), executor.execute(workload)).await;
        assert!(result.is_ok(), "Workload {} should not timeout", i);
        assert!(result.unwrap().is_ok(), "Workload {} should execute", i);
    }
}

#[tokio::test]
async fn e2e_concurrent_workload_submissions() {
    let executor = Arc::new(create_coordinator_executor().await.expect("Create executor"));
    
    let mut handles = vec![];
    
    for i in 0..10 {
        let executor_clone = Arc::clone(&executor);
        let handle = tokio::spawn(async move {
            let mut workload = create_test_workload("cpu_compute");
            workload.metadata.insert("index".to_string(), i.to_string());
            
            timeout(Duration::from_secs(5), executor_clone.execute(workload)).await
        });
        handles.push(handle);
    }
    
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Workload {} should not timeout", i);
        assert!(result.unwrap().is_ok(), "Workload {} should execute", i);
    }
}

// ============================================================================
// E2E: Different Workload Types
// ============================================================================

#[tokio::test]
async fn e2e_cpu_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("cpu_compute");
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "CPU workload should execute");
}

#[tokio::test]
async fn e2e_gpu_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("gpu_compute");
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "GPU workload should execute");
}

#[tokio::test]
async fn e2e_wasm_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("wasm_runtime");
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "WASM workload should execute");
}

#[tokio::test]
async fn e2e_neural_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("neural_compute");
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Neural workload should execute");
}

#[tokio::test]
async fn e2e_container_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("container_runtime");
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Container workload should execute");
}

// ============================================================================
// E2E: Capability Query
// ============================================================================

#[tokio::test]
async fn e2e_query_capabilities() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    
    let result = timeout(Duration::from_secs(5), executor.query_capabilities()).await;
    
    assert!(result.is_ok(), "Capability query should not timeout");
    let capabilities = result.unwrap().expect("Should get capabilities");
    
    assert!(!capabilities.service_id.is_empty(), "Should have service ID");
    assert!(!capabilities.compute_units.is_empty(), "Should have compute units");
    assert!(!capabilities.supported_workload_types.is_empty(), "Should support workload types");
}

// ============================================================================
// E2E: Workload Cancellation
// ============================================================================

#[tokio::test]
async fn e2e_workload_cancellation() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let workload = create_test_workload("cpu_compute");
    let workload_id = workload.workload_id.clone();
    
    // Submit workload
    let submit_result = executor.execute(workload).await;
    assert!(submit_result.is_ok(), "Workload should submit");
    
    // Cancel workload
    let cancel_result = timeout(Duration::from_secs(5), executor.cancel(&workload_id)).await;
    assert!(cancel_result.is_ok(), "Cancel should not timeout");
    assert!(cancel_result.unwrap().is_ok(), "Cancel should succeed");
}

// ============================================================================
// E2E: Resource Requirements
// ============================================================================

#[tokio::test]
async fn e2e_minimal_resources() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.requirements = ResourceRequirements {
        cpu_cores: Some(1),
        memory_bytes: Some(1024 * 1024), // 1MB
        gpu_memory_bytes: None,
        timeout_secs: Some(5),
    };
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Minimal resource workload should execute");
}

#[tokio::test]
async fn e2e_high_resources() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.requirements = ResourceRequirements {
        cpu_cores: Some(8),
        memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
        gpu_memory_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB GPU
        timeout_secs: Some(300),
    };
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "High resource workload should execute");
}

// ============================================================================
// E2E: Priority Handling
// ============================================================================

#[tokio::test]
async fn e2e_high_priority_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.priority = WorkloadPriority::High;
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "High priority workload should execute");
}

#[tokio::test]
async fn e2e_low_priority_workload() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.priority = WorkloadPriority::Low;
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Low priority workload should execute");
}

// ============================================================================
// E2E: Metadata Handling
// ============================================================================

#[tokio::test]
async fn e2e_workload_with_metadata() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    
    workload.metadata.insert("user".to_string(), "test-user".to_string());
    workload.metadata.insert("project".to_string(), "test-project".to_string());
    workload.metadata.insert("environment".to_string(), "test".to_string());
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Workload with metadata should execute");
}

// ============================================================================
// E2E: Edge Cases
// ============================================================================

#[tokio::test]
async fn e2e_empty_workload_data() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.data = vec![]; // Empty data
    
    let result = executor.execute(workload).await;
    assert!(result.is_ok(), "Empty data workload should execute");
}

#[tokio::test]
async fn e2e_large_workload_data() {
    let executor = create_coordinator_executor().await.expect("Create executor");
    let mut workload = create_test_workload("cpu_compute");
    workload.data = vec![0u8; 10 * 1024 * 1024]; // 10MB
    
    let result = timeout(Duration::from_secs(10), executor.execute(workload)).await;
    assert!(result.is_ok(), "Large data workload should not timeout");
    assert!(result.unwrap().is_ok(), "Large data workload should execute");
}

// ============================================================================
// E2E: Standalone Executor Comparison
// ============================================================================

#[tokio::test]
async fn e2e_standalone_vs_coordinator_executor() {
    // Test that both standalone and coordinator executors work
    let standalone = Arc::new(StandaloneExecutor::new());
    let coordinator = create_coordinator_executor().await.expect("Create coordinator executor");
    
    let workload1 = create_test_workload("cpu_compute");
    let workload2 = create_test_workload("cpu_compute");
    
    let result1 = standalone.execute(workload1).await;
    let result2 = coordinator.execute(workload2).await;
    
    assert!(result1.is_ok(), "Standalone executor should work");
    assert!(result2.is_ok(), "Coordinator executor should work");
}

// ============================================================================
// E2E: Full Workflow Simulation
// ============================================================================

#[tokio::test]
async fn e2e_complete_workflow_simulation() {
    // Simulate a complete user workflow:
    // 1. Initialize coordinator
    // 2. Query capabilities
    // 3. Submit multiple workloads
    // 4. Cancel one workload
    // 5. Wait for completion
    
    let executor = Arc::new(create_coordinator_executor().await.expect("Create executor"));
    
    // Step 1: Query capabilities
    let capabilities = executor.query_capabilities().await.expect("Query capabilities");
    assert!(!capabilities.service_id.is_empty());
    
    // Step 2: Submit multiple workloads
    let mut workload_ids = vec![];
    for i in 0..5 {
        let mut workload = create_test_workload("cpu_compute");
        workload.metadata.insert("task".to_string(), format!("task-{}", i));
        workload_ids.push(workload.workload_id.clone());
        
        let result = executor.execute(workload).await;
        assert!(result.is_ok(), "Workload {} should submit", i);
    }
    
    // Step 3: Cancel one workload
    if let Some(id) = workload_ids.first() {
        let cancel_result = executor.cancel(id).await;
        assert!(cancel_result.is_ok(), "Should cancel workload");
    }
    
    // Workflow completed successfully
}

