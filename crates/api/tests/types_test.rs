//! Tests for API Types
//!
//! Testing strategy:
//! - Type structures and validation
//! - Serialization/deserialization
//! - Enum variants
//! - Default implementations

use chrono::Utc;
use std::collections::HashMap;
use toadstool_api::types::*;
use uuid::Uuid;

#[test]
fn test_execution_status_submitted() {
    let status = ExecutionStatus::Submitted;
    match status {
        ExecutionStatus::Submitted => {} // OK - variant matches
        _ => panic!("Expected Submitted"),
    }
}

#[test]
fn test_execution_status_queued() {
    let status = ExecutionStatus::Queued;
    match status {
        ExecutionStatus::Queued => {} // OK - variant matches
        _ => panic!("Expected Queued"),
    }
}

#[test]
fn test_execution_status_running() {
    let status = ExecutionStatus::Running;
    match status {
        ExecutionStatus::Running => {} // OK - variant matches
        _ => panic!("Expected Running"),
    }
}

#[test]
fn test_execution_status_completed() {
    let status = ExecutionStatus::Completed;
    match status {
        ExecutionStatus::Completed => {} // OK - variant matches
        _ => panic!("Expected Completed"),
    }
}

#[test]
fn test_execution_status_failed() {
    let status = ExecutionStatus::Failed;
    match status {
        ExecutionStatus::Failed => {} // OK - variant matches
        _ => panic!("Expected Failed"),
    }
}

#[test]
fn test_execution_status_cancelled() {
    let status = ExecutionStatus::Cancelled;
    match status {
        ExecutionStatus::Cancelled => {} // OK - variant matches
        _ => panic!("Expected Cancelled"),
    }
}

#[test]
fn test_execution_status_timedout() {
    let status = ExecutionStatus::TimedOut;
    match status {
        ExecutionStatus::TimedOut => {} // OK - variant matches
        _ => panic!("Expected TimedOut"),
    }
}

#[test]
fn test_execution_status_paused() {
    let status = ExecutionStatus::Paused;
    match status {
        ExecutionStatus::Paused => {} // OK - variant matches
        _ => panic!("Expected Paused"),
    }
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Running;
    let json = serde_json::to_string(&status).expect("Should serialize");
    assert!(json.contains("running"));
}

#[test]
fn test_execution_status_deserialization() {
    let json = r#""completed""#;
    let status: ExecutionStatus = serde_json::from_str(json).expect("Should deserialize");
    match status {
        ExecutionStatus::Completed => {} // OK - deserialized correctly
        _ => panic!("Expected Completed"),
    }
}

// ============================================================================
// Day 5: API Types Tests - Serialization & Validation
// ============================================================================

#[test]
fn test_all_execution_statuses_serialize() {
    // Test that all execution statuses can be serialized
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
        let json = serde_json::to_string(&status);
        assert!(json.is_ok(), "Status {:?} should serialize", status);
    }
}

#[test]
fn test_all_execution_statuses_roundtrip() {
    // Test serialization roundtrip for all statuses
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
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();

        // Verify it matches
        match (&status, &deserialized) {
            (ExecutionStatus::Submitted, ExecutionStatus::Submitted) => {}
            (ExecutionStatus::Queued, ExecutionStatus::Queued) => {}
            (ExecutionStatus::Running, ExecutionStatus::Running) => {}
            (ExecutionStatus::Completed, ExecutionStatus::Completed) => {}
            (ExecutionStatus::Failed, ExecutionStatus::Failed) => {}
            (ExecutionStatus::Cancelled, ExecutionStatus::Cancelled) => {}
            (ExecutionStatus::TimedOut, ExecutionStatus::TimedOut) => {}
            (ExecutionStatus::Paused, ExecutionStatus::Paused) => {}
            _ => panic!("Status roundtrip failed for {:?}", status),
        }
    }
}

#[test]
fn test_execution_filter_with_status_set() {
    // Test ExecutionFilter with status set
    let filter = ExecutionFilter {
        status: Some(ExecutionStatus::Running),
        ..Default::default()
    };

    assert!(filter.status.is_some());
    assert_eq!(filter.status.unwrap(), ExecutionStatus::Running);
}

#[test]
fn test_execution_filter_with_runtime_type() {
    // Test ExecutionFilter with runtime type set
    let filter = ExecutionFilter {
        runtime_type: Some(toadstool::RuntimeType::Wasm),
        ..Default::default()
    };

    assert!(filter.runtime_type.is_some());
}

#[test]
fn test_api_config_initialization() {
    // Test ApiConfig initializes correctly
    let config = ApiConfig::default();

    // Config should initialize without errors
    let _ = config;
}

#[test]
fn test_api_metrics_initialization() {
    // Test ApiMetrics initializes correctly
    let metrics = ApiMetrics::default();

    // Metrics should start at zero/empty
    let _ = metrics;
}

#[test]
fn test_execution_info_creation() {
    // Test creating ExecutionInfo with all fields
    let exec_id = Uuid::new_v4();
    let exec_info = ExecutionInfo {
        execution_id: exec_id,
        status: ExecutionStatus::Running,
        runtime_type: toadstool::RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(50.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(exec_info.execution_id, exec_id);
    assert_eq!(exec_info.status, ExecutionStatus::Running);
}

#[test]
fn test_execution_info_with_metadata() {
    // Test ExecutionInfo with metadata
    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "test".to_string());
    metadata.insert("priority".to_string(), "high".to_string());

    let exec_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        runtime_type: toadstool::RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(25.0),
        error_message: None,
        resource_usage: None,
        metadata: metadata.clone(),
    };

    assert_eq!(exec_info.metadata.len(), 2);
    assert_eq!(exec_info.metadata.get("user"), Some(&"test".to_string()));
}

#[test]
fn test_execution_info_with_error() {
    // Test ExecutionInfo with error message
    let exec_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        runtime_type: toadstool::RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(1000),
        progress: Some(75.0),
        error_message: Some("Test error".to_string()),
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(exec_info.status, ExecutionStatus::Failed);
    assert!(exec_info.error_message.is_some());
    assert_eq!(exec_info.error_message.unwrap(), "Test error");
}

#[test]
fn test_execution_info_completed() {
    // Test completed execution has all fields set
    let exec_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        runtime_type: toadstool::RuntimeType::Wasm,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(5000),
        progress: Some(100.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(exec_info.status, ExecutionStatus::Completed);
    assert_eq!(exec_info.progress, Some(100.0));
    assert!(exec_info.completed_at.is_some());
    assert!(exec_info.duration_ms.is_some());
}

#[test]
fn test_execution_progress_values() {
    // Test various progress values
    let progress_values = vec![0.0, 25.0, 50.0, 75.0, 100.0];

    for progress in progress_values {
        let exec_info = ExecutionInfo {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Running,
            runtime_type: toadstool::RuntimeType::Native,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(progress),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        assert_eq!(exec_info.progress, Some(progress));
    }
}

#[test]
fn test_execution_duration_tracking() {
    // Test execution duration in milliseconds
    let durations = vec![100, 1000, 5000, 10000, 60000];

    for duration in durations {
        let exec_info = ExecutionInfo {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Completed,
            runtime_type: toadstool::RuntimeType::Container,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            duration_ms: Some(duration),
            progress: Some(100.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        assert_eq!(exec_info.duration_ms, Some(duration));
    }
}

#[test]
fn test_multiple_runtime_types() {
    // Test ExecutionInfo with different runtime types
    let runtime_types = vec![
        toadstool::RuntimeType::Native,
        toadstool::RuntimeType::Wasm,
        toadstool::RuntimeType::Container,
        toadstool::RuntimeType::Python,
    ];

    for runtime_type in runtime_types {
        let exec_info = ExecutionInfo {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Running,
            runtime_type: runtime_type.clone(),
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(50.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        assert_eq!(exec_info.runtime_type, runtime_type);
    }
}

#[test]
fn test_execution_info_structure() {
    let exec_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        runtime_type: toadstool::RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(50.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    assert_eq!(exec_info.status, ExecutionStatus::Running);
    assert_eq!(exec_info.progress, Some(50.0));
    assert!(exec_info.started_at.is_some());
    assert!(exec_info.completed_at.is_none());
}

#[test]
fn test_workload_spec_native() {
    let spec = WorkloadSpec::Native {
        executable: "echo".to_string(),
        args: vec!["hello".to_string()],
    };

    match spec {
        WorkloadSpec::Native { executable, args } => {
            assert_eq!(executable, "echo");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Native workload"),
    }
}

#[test]
fn test_workload_spec_container() {
    let spec = WorkloadSpec::Container {
        image: "ubuntu:latest".to_string(),
        command: Some(vec!["bash".to_string()]),
        args: None,
    };

    match spec {
        WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "ubuntu:latest");
        }
        _ => panic!("Expected Container workload"),
    }
}

#[test]
fn test_workload_spec_wasm() {
    let spec = WorkloadSpec::Wasm {
        module: "app.wasm".to_string(),
        function: "main".to_string(),
        args: vec![],
    };

    match spec {
        WorkloadSpec::Wasm {
            module, function, ..
        } => {
            assert_eq!(module, "app.wasm");
            assert_eq!(function, "main");
        }
        _ => panic!("Expected Wasm workload"),
    }
}

#[test]
fn test_workload_spec_python() {
    let spec = WorkloadSpec::Python {
        script: "app.py".to_string(),
        requirements: Some(vec!["requests".to_string()]),
    };

    match spec {
        WorkloadSpec::Python {
            script,
            requirements,
        } => {
            assert_eq!(script, "app.py");
            assert!(requirements.is_some());
        }
        _ => panic!("Expected Python workload"),
    }
}

#[test]
fn test_workload_spec_gpu() {
    let spec = WorkloadSpec::Gpu {
        kernel: "compute.cu".to_string(),
        platform: "CUDA".to_string(),
        args: vec![],
    };

    match spec {
        WorkloadSpec::Gpu { platform, .. } => {
            assert_eq!(platform, "CUDA");
        }
        _ => panic!("Expected Gpu workload"),
    }
}

#[test]
fn test_resource_requirements_structure() {
    let reqs = ResourceRequirements {
        cpu_cores: Some(2.0),
        memory_mb: Some(2048),
        storage_mb: Some(10240),
        gpu_count: Some(1),
        network_mbps: Some(100),
    };

    assert_eq!(reqs.cpu_cores, Some(2.0));
    assert_eq!(reqs.memory_mb, Some(2048));
    assert_eq!(reqs.gpu_count, Some(1));
}

#[test]
fn test_resource_requirements_minimal() {
    let reqs = ResourceRequirements {
        cpu_cores: Some(0.5),
        memory_mb: Some(512),
        storage_mb: None,
        gpu_count: None,
        network_mbps: None,
    };

    assert_eq!(reqs.cpu_cores, Some(0.5));
    assert!(reqs.gpu_count.is_none());
}

#[test]
fn test_execution_filter_default() {
    let filter = ExecutionFilter::default();

    assert!(filter.status.is_none());
    assert!(filter.runtime_type.is_none());
}

#[test]
fn test_health_response_structure() {
    let health = HealthResponse {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        uptime_seconds: 60,
        timestamp: Utc::now(),
        checks: vec![],
    };

    assert_eq!(health.status, "healthy");
    assert_eq!(health.version, "1.0.0");
    assert_eq!(health.uptime_seconds, 60);
}

#[test]
fn test_api_metrics_default() {
    let metrics = ApiMetrics::default();

    assert_eq!(metrics.total_requests, 0);
    assert_eq!(metrics.successful_requests, 0);
    assert_eq!(metrics.failed_requests, 0);
}

#[test]
fn test_node_status_healthy() {
    let status = NodeStatus::Healthy;
    match status {
        NodeStatus::Healthy => {} // OK - variant matches
        _ => panic!("Expected Healthy"),
    }
}

#[test]
fn test_node_status_degraded() {
    let status = NodeStatus::Degraded;
    match status {
        NodeStatus::Degraded => {} // OK - variant matches
        _ => panic!("Expected Degraded"),
    }
}

#[test]
fn test_node_status_unhealthy() {
    let status = NodeStatus::Unhealthy;
    match status {
        NodeStatus::Unhealthy => {} // OK - variant matches
        _ => panic!("Expected Unhealthy"),
    }
}

#[test]
fn test_node_resources_structure() {
    let resources = NodeResources {
        cpu_cores: 8,
        memory_gb: 16,
        storage_gb: 500,
        gpu_count: 1,
    };

    assert_eq!(resources.cpu_cores, 8);
    assert_eq!(resources.memory_gb, 16);
    assert_eq!(resources.storage_gb, 500);
    assert_eq!(resources.gpu_count, 1);
}

#[test]
fn test_cluster_capacity_structure() {
    let capacity = ClusterCapacity {
        cpu_cores: 32,
        memory_gb: 64,
        storage_gb: 2000,
        gpu_count: 4,
    };

    assert_eq!(capacity.cpu_cores, 32);
    assert_eq!(capacity.memory_gb, 64);
    assert_eq!(capacity.storage_gb, 2000);
    assert_eq!(capacity.gpu_count, 4);
}

#[test]
fn test_workload_spec_serialization() {
    let spec = WorkloadSpec::Native {
        executable: "test".to_string(),
        args: vec![],
    };

    let json = serde_json::to_string(&spec).expect("Should serialize");
    assert!(json.contains("Native"));
    assert!(json.contains("test"));
}

#[test]
fn test_execution_status_equality() {
    let status1 = ExecutionStatus::Running;
    let status2 = ExecutionStatus::Running;
    assert_eq!(status1, status2);
}

#[test]
fn test_execution_status_inequality() {
    let status1 = ExecutionStatus::Running;
    let status2 = ExecutionStatus::Completed;
    assert_ne!(status1, status2);
}

#[test]
fn test_node_status_serialization() {
    let status = NodeStatus::Healthy;
    let json = serde_json::to_string(&status).expect("Should serialize");
    // NodeStatus uses snake_case serialization
    assert!(json.contains("healthy") || json.contains("Healthy"));
}
