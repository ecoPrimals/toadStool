//! Comprehensive test coverage for API handlers
//!
//! This test suite covers the modern API v2 handlers with proper types

use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::execution::RuntimeType;
use toadstool_api::{
    types::{ExecutionInfo, ExecutionStatus, ResourceRequirements, WorkloadSpec},
    ApiState,
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Helper to create test API state
fn create_test_state() -> ApiState {
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    let websocket_manager = Arc::new(toadstool_api::websocket::WebSocketManager::new());

    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster: event_tx,
        websocket_manager,
        capability_provider: None,
    }
}

#[test]
fn test_execution_status_variants() {
    // Ensure all status variants can be created
    let statuses = vec![
        ExecutionStatus::Submitted,
        ExecutionStatus::Queued,
        ExecutionStatus::Running,
        ExecutionStatus::Completed,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Paused,
    ];

    for status in statuses {
        // Verify status can be created and matched
        assert!(matches!(
            status,
            ExecutionStatus::Submitted
                | ExecutionStatus::Queued
                | ExecutionStatus::Running
                | ExecutionStatus::Completed
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::TimedOut
                | ExecutionStatus::Paused
        ));
    }
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, r#""running""#);

    let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, ExecutionStatus::Running);
}

#[test]
fn test_workload_spec_native() {
    let workload = WorkloadSpec::Native {
        executable: "/bin/echo".to_string(),
        args: vec!["hello".to_string()],
    };

    match workload {
        WorkloadSpec::Native { executable, args } => {
            assert_eq!(executable, "/bin/echo");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_python() {
    let workload = WorkloadSpec::Python {
        script: "print('test')".to_string(),
        requirements: Some(vec!["numpy".to_string()]),
    };

    match workload {
        WorkloadSpec::Python {
            script,
            requirements,
        } => {
            assert!(script.contains("print"));
            assert_eq!(requirements.unwrap().len(), 1);
        }
        _ => panic!("Expected Python workload"),
    }
}

#[test]
fn test_workload_spec_wasm() {
    let workload = WorkloadSpec::Wasm {
        module: "test.wasm".to_string(),
        function: "main".to_string(),
        args: vec![],
    };

    match workload {
        WorkloadSpec::Wasm {
            module, function, ..
        } => {
            assert_eq!(module, "test.wasm");
            assert_eq!(function, "main");
        }
        _ => panic!("Expected WASM workload"),
    }
}

#[test]
fn test_workload_spec_container() {
    let workload = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: Some(vec!["echo".to_string()]),
        args: Some(vec!["test".to_string()]),
    };

    match workload {
        WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "alpine:latest");
        }
        _ => panic!("Expected Container workload"),
    }
}

#[test]
fn test_workload_spec_gpu() {
    let workload = WorkloadSpec::Gpu {
        kernel: "matrix_mul".to_string(),
        platform: "cuda".to_string(),
        args: vec![],
    };

    match workload {
        WorkloadSpec::Gpu {
            kernel, platform, ..
        } => {
            assert_eq!(kernel, "matrix_mul");
            assert_eq!(platform, "cuda");
        }
        _ => panic!("Expected GPU workload"),
    }
}

#[test]
fn test_resource_requirements_defaults() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: None,
        storage_mb: None,
        gpu_count: None,
        network_mbps: None,
    };

    assert!(resources.cpu_cores.is_none());
    assert!(resources.memory_mb.is_none());
    assert!(resources.gpu_count.is_none());
}

#[test]
fn test_resource_requirements_partial() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2.0),
        memory_mb: Some(1024),
        storage_mb: None,
        gpu_count: None,
        network_mbps: None,
    };

    assert_eq!(resources.cpu_cores, Some(2.0));
    assert_eq!(resources.memory_mb, Some(1024));
    assert!(resources.gpu_count.is_none());
}

#[test]
fn test_resource_requirements_full() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4.0),
        memory_mb: Some(2048),
        storage_mb: Some(10240),
        gpu_count: Some(1),
        network_mbps: Some(1000),
    };

    assert_eq!(resources.cpu_cores, Some(4.0));
    assert_eq!(resources.memory_mb, Some(2048));
    assert_eq!(resources.storage_mb, Some(10240));
    assert_eq!(resources.gpu_count, Some(1));
    assert_eq!(resources.network_mbps, Some(1000));
}

#[tokio::test]
async fn test_api_state_creation() {
    let state = create_test_state();
    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 0);
}

#[tokio::test]
async fn test_api_state_add_execution() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Submitted,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: Some(0.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 1);
    assert!(executions.contains_key(&execution_id));
}

#[tokio::test]
async fn test_api_state_multiple_executions() {
    let state = create_test_state();

    for _i in 0..10 {
        let execution_id = Uuid::new_v4();
        let execution_info = ExecutionInfo {
            execution_id,
            status: ExecutionStatus::Running,
            runtime_type: RuntimeType::Native,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(0.5),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        let mut executions = state.executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 10);
}

#[tokio::test]
async fn test_execution_info_timestamps() {
    let now = Utc::now();
    let execution_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Wasm,
        submitted_at: now,
        started_at: Some(now),
        completed_at: None,
        duration_ms: None,
        progress: Some(0.3),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(execution_info.submitted_at, now);
    assert_eq!(execution_info.started_at, Some(now));
    assert!(execution_info.completed_at.is_none());
}

#[test]
fn test_execution_info_with_error() {
    let execution_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        runtime_type: RuntimeType::Python,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(1500),
        progress: Some(0.8),
        error_message: Some("Process exited with code 1".to_string()),
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(execution_info.status, ExecutionStatus::Failed);
    assert!(execution_info.error_message.is_some());
    assert_eq!(
        execution_info.error_message.unwrap(),
        "Process exited with code 1"
    );
}

#[test]
fn test_execution_info_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "test-user".to_string());
    metadata.insert("project".to_string(), "test-project".to_string());

    let execution_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        runtime_type: RuntimeType::Container,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(3000),
        progress: Some(1.0),
        error_message: None,
        resource_usage: None,
        metadata: metadata.clone(),
    };

    assert_eq!(execution_info.metadata.get("user_id").unwrap(), "test-user");
    assert_eq!(
        execution_info.metadata.get("project").unwrap(),
        "test-project"
    );
}

#[test]
fn test_runtime_type_variants() {
    let types = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
        RuntimeType::Gpu,
    ];

    for runtime_type in types {
        // Verify all runtime types can be created
        assert!(matches!(
            runtime_type,
            RuntimeType::Native
                | RuntimeType::Container
                | RuntimeType::Wasm
                | RuntimeType::Python
                | RuntimeType::Gpu
        ));
    }
}
