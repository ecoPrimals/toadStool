//! Comprehensive unit tests for client types module
//! Target: crates/client/src/client/types.rs
//! Focus: Type conversions, enums, and struct operations

use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use toadstool_client::*;
use uuid::Uuid;

// ============================================================================
// WorkloadSubmission Tests
// ============================================================================

#[test]
fn test_workload_submission_creation() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            working_dir: None,
        },
        runtime_hint: Some("native".to_string()),
        priority: Some(JobPriority::Normal),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert!(matches!(
        submission.workload_type,
        WorkloadType::Native { .. }
    ));
    assert_eq!(submission.runtime_hint, Some("native".to_string()));
}

#[test]
fn test_workload_submission_clone() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/ls".to_string(),
            args: vec![],
            working_dir: None,
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    let cloned = submission.clone();
    assert!(matches!(cloned.workload_type, WorkloadType::Native { .. }));
}

#[test]
fn test_workload_submission_debug() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/test".to_string(),
            args: vec![],
            working_dir: None,
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    let debug_str = format!("{:?}", submission);
    assert!(debug_str.contains("WorkloadSubmission"));
}

// ============================================================================
// WorkloadType Tests
// ============================================================================

#[test]
fn test_workload_type_native() {
    let workload = WorkloadType::Native {
        executable: "/usr/bin/python3".to_string(),
        args: vec!["script.py".to_string()],
        working_dir: Some("/home/user".to_string()),
    };

    assert!(matches!(workload, WorkloadType::Native { .. }));
    if let WorkloadType::Native {
        executable,
        args,
        working_dir,
    } = workload
    {
        assert_eq!(executable, "/usr/bin/python3");
        assert_eq!(args.len(), 1);
        assert_eq!(working_dir, Some("/home/user".to_string()));
    }
}

#[test]
fn test_workload_type_container() {
    let workload = WorkloadType::Container {
        image: "nginx:latest".to_string(),
        command: Some(vec!["/bin/sh".to_string()]),
        args: Some(vec!["-c".to_string(), "echo hello".to_string()]),
        working_dir: Some("/app".to_string()),
    };

    assert!(matches!(workload, WorkloadType::Container { .. }));
    if let WorkloadType::Container {
        image,
        command,
        args,
        working_dir,
    } = workload
    {
        assert_eq!(image, "nginx:latest");
        assert!(command.is_some());
        assert!(args.is_some());
        assert_eq!(working_dir, Some("/app".to_string()));
    }
}

#[test]
fn test_workload_type_wasm() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number
    let workload = WorkloadType::Wasm {
        module_data: module_data.clone(),
        args: vec!["arg1".to_string()],
    };

    assert!(matches!(workload, WorkloadType::Wasm { .. }));
    if let WorkloadType::Wasm {
        module_data: data,
        args,
    } = workload
    {
        assert_eq!(data, module_data);
        assert_eq!(args.len(), 1);
    }
}

#[test]
fn test_workload_type_python() {
    let workload = WorkloadType::Python {
        script: "print('hello')".to_string(),
        requirements: vec!["numpy".to_string(), "pandas".to_string()],
    };

    assert!(matches!(workload, WorkloadType::Python { .. }));
    if let WorkloadType::Python {
        script,
        requirements,
    } = workload
    {
        assert!(script.contains("print"));
        assert_eq!(requirements.len(), 2);
    }
}

#[test]
fn test_workload_type_custom() {
    let data = serde_json::json!({"custom": "data", "value": 42});
    let workload = WorkloadType::Custom {
        workload_data: data.clone(),
    };

    assert!(matches!(workload, WorkloadType::Custom { .. }));
}

#[test]
fn test_workload_type_serialization() {
    let workload = WorkloadType::Native {
        executable: "/bin/test".to_string(),
        args: vec!["arg".to_string()],
        working_dir: None,
    };

    let serialized = serde_json::to_string(&workload);
    assert!(serialized.is_ok());

    let deserialized: Result<WorkloadType, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_workload_type_clone() {
    let workload = WorkloadType::Container {
        image: "alpine:latest".to_string(),
        command: None,
        args: None,
        working_dir: None,
    };

    let cloned = workload.clone();
    assert!(matches!(cloned, WorkloadType::Container { .. }));
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let resources = ResourceRequirements::default();

    assert_eq!(resources.cpu_cores, None);
    assert_eq!(resources.memory_mb, None);
    assert_eq!(resources.disk_mb, None);
    assert_eq!(resources.gpu_required, None);
}

#[test]
fn test_resource_requirements_custom() {
    let resources = ResourceRequirements {
        cpu_cores: Some(8),
        memory_mb: Some(16384),
        disk_mb: Some(102400),
        gpu_required: Some(true),
    };

    assert_eq!(resources.cpu_cores, Some(8));
    assert_eq!(resources.memory_mb, Some(16384));
    assert_eq!(resources.disk_mb, Some(102400));
    assert_eq!(resources.gpu_required, Some(true));
}

#[test]
fn test_resource_requirements_partial() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: None,
        gpu_required: None,
    };

    assert_eq!(resources.cpu_cores, Some(4));
    assert_eq!(resources.memory_mb, Some(8192));
    assert_eq!(resources.disk_mb, None);
    assert_eq!(resources.gpu_required, None);
}

#[test]
fn test_resource_requirements_clone() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(4096),
        disk_mb: Some(10240),
        gpu_required: Some(false),
    };

    let cloned = resources.clone();
    assert_eq!(cloned, resources);
}

#[test]
fn test_resource_requirements_serialization() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: Some(20480),
        gpu_required: Some(true),
    };

    let serialized = serde_json::to_string(&resources);
    assert!(serialized.is_ok());

    let deserialized: Result<ResourceRequirements, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.unwrap(), resources);
}

#[test]
fn test_resource_requirements_debug() {
    let resources = ResourceRequirements {
        cpu_cores: Some(16),
        memory_mb: Some(65536),
        disk_mb: Some(1048576),
        gpu_required: Some(true),
    };

    let debug_str = format!("{:?}", resources);
    assert!(debug_str.contains("ResourceRequirements"));
}

// ============================================================================
// ExecutionInfo Tests
// ============================================================================

#[test]
fn test_execution_info_creation() {
    let execution_id = Uuid::new_v4();
    let now = SystemTime::now();

    let info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Pending,
        submitted_at: now,
        started_at: None,
        completed_at: None,
        runtime_type: Some("native".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    assert_eq!(info.execution_id, execution_id);
    assert!(matches!(info.status, ExecutionStatus::Pending));
    assert_eq!(info.runtime_type, Some("native".to_string()));
}

#[test]
fn test_execution_info_clone() {
    let execution_id = Uuid::new_v4();
    let info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Running,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: None,
        runtime_type: Some("container".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    let cloned = info.clone();
    assert_eq!(cloned.execution_id, info.execution_id);
}

#[test]
fn test_execution_info_serialization() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: Some(SystemTime::now()),
        runtime_type: Some("python".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    let serialized = serde_json::to_string(&info);
    assert!(serialized.is_ok());
}

#[test]
fn test_execution_info_debug() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: Some(SystemTime::now()),
        runtime_type: Some("wasm".to_string()),
        error_message: Some("Test error".to_string()),
        output: None,
        metrics: None,
    };

    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("ExecutionInfo"));
}

// ============================================================================
// ExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_pending() {
    let status = ExecutionStatus::Pending;
    assert!(matches!(status, ExecutionStatus::Pending));
}

#[test]
fn test_execution_status_queued() {
    let status = ExecutionStatus::Queued;
    assert!(matches!(status, ExecutionStatus::Queued));
}

#[test]
fn test_execution_status_running() {
    let status = ExecutionStatus::Running;
    assert!(matches!(status, ExecutionStatus::Running));
}

#[test]
fn test_execution_status_completed() {
    let status = ExecutionStatus::Completed;
    assert!(matches!(status, ExecutionStatus::Completed));
}

#[test]
fn test_execution_status_failed() {
    let status = ExecutionStatus::Failed;
    assert!(matches!(status, ExecutionStatus::Failed));
}

#[test]
fn test_execution_status_cancelled() {
    let status = ExecutionStatus::Cancelled;
    assert!(matches!(status, ExecutionStatus::Cancelled));
}

#[test]
fn test_execution_status_timeout() {
    let status = ExecutionStatus::Timeout;
    assert!(matches!(status, ExecutionStatus::Timeout));
}

#[test]
fn test_execution_status_serialization() {
    let statuses = vec![
        ExecutionStatus::Pending,
        ExecutionStatus::Queued,
        ExecutionStatus::Running,
        ExecutionStatus::Completed,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
        ExecutionStatus::Timeout,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status);
        assert!(serialized.is_ok());

        let deserialized: Result<ExecutionStatus, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
    }
}

#[test]
fn test_execution_status_clone() {
    let status = ExecutionStatus::Running;
    let cloned = status.clone();
    assert!(matches!(cloned, ExecutionStatus::Running));
}

#[test]
fn test_execution_status_debug() {
    let status = ExecutionStatus::Completed;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Completed"));
}

// ============================================================================
// ExecutionOutput Tests
// ============================================================================

#[test]
fn test_execution_output_creation() {
    let output = ExecutionOutput {
        stdout: Some("Hello, World!".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert_eq!(output.stdout, Some("Hello, World!".to_string()));
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_error() {
    let output = ExecutionOutput {
        stdout: Some("Some output".to_string()),
        stderr: Some("Error occurred".to_string()),
        exit_code: Some(1),
        artifacts: vec![],
    };

    assert_eq!(output.stderr, Some("Error occurred".to_string()));
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_artifacts() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        artifacts: vec!["artifact1.txt".to_string(), "artifact2.bin".to_string()],
    };

    assert_eq!(output.artifacts.len(), 2);
}

#[test]
fn test_execution_output_clone() {
    let output = ExecutionOutput {
        stdout: Some("output".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec!["file.txt".to_string()],
    };

    let cloned = output.clone();
    assert_eq!(cloned.stdout, output.stdout);
    assert_eq!(cloned.artifacts.len(), output.artifacts.len());
}

#[test]
fn test_execution_output_serialization() {
    let output = ExecutionOutput {
        stdout: Some("test".to_string()),
        stderr: Some("error".to_string()),
        exit_code: Some(1),
        artifacts: vec!["artifact.txt".to_string()],
    };

    let serialized = serde_json::to_string(&output);
    assert!(serialized.is_ok());

    let deserialized: Result<ExecutionOutput, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_execution_output_debug() {
    let output = ExecutionOutput {
        stdout: Some("debug".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    let debug_str = format!("{:?}", output);
    assert!(debug_str.contains("ExecutionOutput"));
}

// ============================================================================
// ExecutionMetrics Tests
// ============================================================================

#[test]
fn test_execution_metrics_creation() {
    let metrics = ExecutionMetrics {
        duration_ms: 1500,
        cpu_usage_percent: 85.5,
        memory_peak_bytes: 1024 * 1024 * 512, // 512 MB
        network_bytes_sent: 1024 * 100,
        network_bytes_received: 1024 * 200,
    };

    assert_eq!(metrics.duration_ms, 1500);
    assert_eq!(metrics.cpu_usage_percent, 85.5);
    assert_eq!(metrics.memory_peak_bytes, 1024 * 1024 * 512);
}

#[test]
fn test_execution_metrics_clone() {
    let metrics = ExecutionMetrics {
        duration_ms: 2000,
        cpu_usage_percent: 50.0,
        memory_peak_bytes: 1024 * 1024 * 256,
        network_bytes_sent: 5000,
        network_bytes_received: 10000,
    };

    let cloned = metrics.clone();
    assert_eq!(cloned.duration_ms, metrics.duration_ms);
    assert_eq!(cloned.cpu_usage_percent, metrics.cpu_usage_percent);
}

#[test]
fn test_execution_metrics_serialization() {
    let metrics = ExecutionMetrics {
        duration_ms: 3000,
        cpu_usage_percent: 75.0,
        memory_peak_bytes: 1024 * 1024 * 1024,
        network_bytes_sent: 50000,
        network_bytes_received: 100000,
    };

    let serialized = serde_json::to_string(&metrics);
    assert!(serialized.is_ok());

    let deserialized: Result<ExecutionMetrics, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_execution_metrics_debug() {
    let metrics = ExecutionMetrics {
        duration_ms: 500,
        cpu_usage_percent: 25.0,
        memory_peak_bytes: 1024 * 1024 * 128,
        network_bytes_sent: 1000,
        network_bytes_received: 2000,
    };

    let debug_str = format!("{:?}", metrics);
    assert!(debug_str.contains("ExecutionMetrics"));
}

// ============================================================================
// ToadStoolEvent Tests
// ============================================================================

#[test]
fn test_event_execution_status_changed() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: Uuid::new_v4().to_string(),
        status: "completed".to_string(),
    };
    assert!(matches!(
        event,
        ToadStoolEvent::ExecutionStatusChanged { .. }
    ));
}

#[test]
fn test_event_cluster_health_changed() {
    let event = ToadStoolEvent::ClusterHealthChanged { healthy: true };
    assert!(matches!(event, ToadStoolEvent::ClusterHealthChanged { .. }));
}

#[test]
fn test_event_clone() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: "id-123".to_string(),
        status: "running".to_string(),
    };
    let cloned = event.clone();
    assert!(matches!(
        cloned,
        ToadStoolEvent::ExecutionStatusChanged { .. }
    ));
}

#[test]
fn test_event_serialization() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: Uuid::new_v4().to_string(),
        status: "failed".to_string(),
    };
    let serialized = serde_json::to_string(&event);
    assert!(serialized.is_ok());
}

#[test]
fn test_event_debug() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: "id".to_string(),
        status: "running".to_string(),
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ExecutionStatusChanged"));
}

// ============================================================================
// ClusterStatus Tests (from types.rs)
// ============================================================================

#[test]
fn test_cluster_status_creation() {
    let status = ClusterStatus {
        total_nodes: 5,
        healthy_nodes: 4,
        cluster_load: 0.65,
        active_executions: 15,
        available_runtimes: vec!["native".to_string(), "container".to_string()],
    };

    assert_eq!(status.total_nodes, 5);
    assert_eq!(status.healthy_nodes, 4);
    assert_eq!(status.cluster_load, 0.65);
    assert_eq!(status.active_executions, 15);
    assert_eq!(status.available_runtimes.len(), 2);
}

#[test]
fn test_cluster_status_serialization() {
    let status = ClusterStatus {
        total_nodes: 3,
        healthy_nodes: 3,
        cluster_load: 0.45,
        active_executions: 8,
        available_runtimes: vec!["python".to_string()],
    };

    let serialized = serde_json::to_string(&status);
    assert!(serialized.is_ok());
}

#[test]
fn test_cluster_status_clone() {
    let status = ClusterStatus {
        total_nodes: 10,
        healthy_nodes: 8,
        cluster_load: 0.75,
        active_executions: 30,
        available_runtimes: vec!["wasm".to_string(), "gpu".to_string()],
    };

    let cloned = status.clone();
    assert_eq!(cloned.total_nodes, status.total_nodes);
    assert_eq!(cloned.healthy_nodes, status.healthy_nodes);
}

#[test]
fn test_cluster_status_debug() {
    let status = ClusterStatus {
        total_nodes: 2,
        healthy_nodes: 2,
        cluster_load: 0.25,
        active_executions: 5,
        available_runtimes: vec!["native".to_string()],
    };

    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("ClusterStatus"));
}

#[test]
fn test_cluster_status_high_load() {
    let status = ClusterStatus {
        total_nodes: 20,
        healthy_nodes: 18,
        cluster_load: 0.95,
        active_executions: 100,
        available_runtimes: vec![
            "native".to_string(),
            "container".to_string(),
            "wasm".to_string(),
            "python".to_string(),
            "gpu".to_string(),
        ],
    };

    assert!(status.cluster_load > 0.9);
    assert_eq!(status.available_runtimes.len(), 5);
}
