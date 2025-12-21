//! Comprehensive tests for client types and workload submission
//!
//! Goal: Increase client coverage from 0% to 70%+

use chrono::Utc;
use toadstool_client::{
    ClusterStatus, ExecutionInfo, ExecutionMetrics, ExecutionOutput, ExecutionStatus, JobPriority,
    ResourceRequirements, ToadStoolEvent, WorkloadType,
};
use uuid::Uuid;

// ==================== Resource Requirements Tests ====================

#[test]
fn test_resource_requirements_default() {
    let resources = ResourceRequirements::default();
    assert!(resources.cpu_cores.is_none());
    assert!(resources.memory_mb.is_none());
}

#[test]
fn test_resource_requirements_full() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: Some(102400),
        gpu_required: Some(true),
    };

    assert_eq!(resources.cpu_cores, Some(4));
    assert_eq!(resources.memory_mb, Some(8192));
    assert_eq!(resources.disk_mb, Some(102400));
    assert_eq!(resources.gpu_required, Some(true));
}

#[test]
fn test_resource_requirements_minimal_cpu() {
    let resources = ResourceRequirements {
        cpu_cores: Some(1),
        ..Default::default()
    };

    assert_eq!(resources.cpu_cores, Some(1));
}

#[test]
fn test_resource_requirements_gpu_not_required() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(4096),
        disk_mb: Some(10240),
        gpu_required: Some(false),
    };

    assert_eq!(resources.gpu_required, Some(false));
}

// ==================== Workload Type Tests ====================

#[test]
fn test_native_workload_type() {
    let workload = WorkloadType::Native {
        executable: "/bin/echo".to_string(),
        args: vec!["hello".to_string()],
        working_dir: Some("/tmp".to_string()),
    };

    match workload {
        WorkloadType::Native {
            executable,
            args,
            working_dir,
        } => {
            assert_eq!(executable, "/bin/echo");
            assert_eq!(args.len(), 1);
            assert_eq!(working_dir, Some("/tmp".to_string()));
        }
        _ => panic!("Expected native workload"),
    }
}

#[test]
fn test_native_workload_no_workdir() {
    let workload = WorkloadType::Native {
        executable: "/bin/ls".to_string(),
        args: vec![],
        working_dir: None,
    };

    match workload {
        WorkloadType::Native { working_dir, .. } => {
            assert!(working_dir.is_none());
        }
        _ => panic!("Expected native workload"),
    }
}

#[test]
fn test_container_workload_type() {
    let workload = WorkloadType::Container {
        image: "alpine:latest".to_string(),
        command: Some(vec!["sh".to_string()]),
        args: Some(vec!["-c".to_string(), "echo test".to_string()]),
        working_dir: Some("/app".to_string()),
    };

    match workload {
        WorkloadType::Container {
            image,
            command,
            args,
            working_dir,
        } => {
            assert_eq!(image, "alpine:latest");
            assert!(command.is_some());
            assert!(args.is_some());
            assert_eq!(working_dir, Some("/app".to_string()));
        }
        _ => panic!("Expected container workload"),
    }
}

#[test]
fn test_wasm_workload_type() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6D]; // WASM magic

    let workload = WorkloadType::Wasm {
        module_data: module_data.clone(),
        args: vec!["arg1".to_string()],
    };

    match workload {
        WorkloadType::Wasm {
            module_data: data,
            args,
        } => {
            assert_eq!(data, module_data);
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected WASM workload"),
    }
}

#[test]
fn test_python_workload_type() {
    let script = "print('Hello, World!')".to_string();
    let requirements = vec!["numpy".to_string(), "requests".to_string()];

    let workload = WorkloadType::Python {
        script: script.clone(),
        requirements: requirements.clone(),
    };

    match workload {
        WorkloadType::Python {
            script: s,
            requirements: r,
        } => {
            assert_eq!(s, script);
            assert_eq!(r, requirements);
        }
        _ => panic!("Expected Python workload"),
    }
}

#[test]
fn test_custom_workload_type() {
    let data = serde_json::json!({"custom_field": "value"});

    let workload = WorkloadType::Custom {
        workload_data: data.clone(),
    };

    match workload {
        WorkloadType::Custom { workload_data } => {
            assert_eq!(workload_data, data);
        }
        _ => panic!("Expected custom workload"),
    }
}

// ==================== Execution Status Tests ====================

#[test]
fn test_execution_status_variants() {
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
        // Verify all variants are constructible
        let _ = format!("{:?}", status);
    }
}

// ==================== Execution Info Tests ====================

#[test]
fn test_execution_info_creation() {
    let execution_id = Uuid::new_v4();
    let now = Utc::now();

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
}

#[test]
fn test_execution_info_with_error() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        runtime_type: None,
        error_message: Some("Execution failed".to_string()),
        output: None,
        metrics: None,
    };

    assert!(info.error_message.is_some());
    assert_eq!(info.error_message.unwrap(), "Execution failed");
}

#[test]
fn test_execution_info_with_output() {
    let output = ExecutionOutput {
        stdout: Some("Hello".to_string()),
        stderr: Some("Warning".to_string()),
        exit_code: Some(0),
        artifacts: vec!["output.txt".to_string()],
    };

    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        runtime_type: Some("container".to_string()),
        error_message: None,
        output: Some(output.clone()),
        metrics: None,
    };

    assert!(info.output.is_some());
}

// ==================== Execution Output Tests ====================

#[test]
fn test_execution_output() {
    let output = ExecutionOutput {
        stdout: Some("Test output".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stdout.is_some());
    assert!(output.stderr.is_none());
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_artifacts() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        artifacts: vec!["file1.txt".to_string(), "file2.dat".to_string()],
    };

    assert_eq!(output.artifacts.len(), 2);
}

// ==================== Execution Metrics Tests ====================

#[test]
fn test_execution_metrics() {
    let metrics = ExecutionMetrics {
        duration_ms: 1500,
        cpu_usage_percent: 45.5,
        memory_peak_bytes: 104857600, // 100MB
        network_bytes_sent: 1024,
        network_bytes_received: 2048,
    };

    assert_eq!(metrics.duration_ms, 1500);
    assert_eq!(metrics.cpu_usage_percent, 45.5);
}

// ==================== ToadStool Event Tests ====================

#[test]
fn test_execution_started_event() {
    let event = ToadStoolEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionStarted { .. } => {
            // Success
        }
        _ => panic!("Expected ExecutionStarted event"),
    }
}

#[test]
fn test_execution_completed_event() {
    let event = ToadStoolEvent::ExecutionCompleted {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionCompleted { status, .. } => {
            assert!(matches!(status, ExecutionStatus::Completed));
        }
        _ => panic!("Expected ExecutionCompleted event"),
    }
}

#[test]
fn test_execution_progress_event() {
    let event = ToadStoolEvent::ExecutionProgress {
        execution_id: Uuid::new_v4(),
        progress_percent: 50.0,
        message: Some("Processing...".to_string()),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionProgress {
            progress_percent,
            message,
            ..
        } => {
            assert_eq!(progress_percent, 50.0);
            assert!(message.is_some());
        }
        _ => panic!("Expected ExecutionProgress event"),
    }
}

#[test]
fn test_cluster_event() {
    let event = ToadStoolEvent::ClusterEvent {
        event_type: "node_joined".to_string(),
        node_id: Some("node-123".to_string()),
        message: "New node joined cluster".to_string(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ClusterEvent { event_type, .. } => {
            assert_eq!(event_type, "node_joined");
        }
        _ => panic!("Expected ClusterEvent"),
    }
}

#[test]
fn test_alert_event() {
    let event = ToadStoolEvent::Alert {
        severity: "warning".to_string(),
        message: "High CPU usage detected".to_string(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::Alert { severity, .. } => {
            assert_eq!(severity, "warning");
        }
        _ => panic!("Expected Alert event"),
    }
}

// ==================== Cluster Status Tests ====================

#[test]
fn test_cluster_status() {
    let status = ClusterStatus {
        total_nodes: 10,
        healthy_nodes: 9,
        cluster_load: 0.65,
        active_executions: 15,
        available_runtimes: vec!["native".to_string(), "container".to_string()],
    };

    assert_eq!(status.total_nodes, 10);
    assert_eq!(status.healthy_nodes, 9);
    assert_eq!(status.available_runtimes.len(), 2);
}

// ==================== Job Priority Tests ====================

#[test]
fn test_job_priority_variants() {
    let priorities = vec![
        JobPriority::Low,
        JobPriority::Normal,
        JobPriority::High,
        JobPriority::Critical,
    ];

    for priority in priorities {
        let _ = format!("{:?}", priority);
    }
}

// ==================== Serialization Tests ====================

#[test]
fn test_workload_type_serialization() {
    let workload = WorkloadType::Native {
        executable: "/bin/echo".to_string(),
        args: vec!["test".to_string()],
        working_dir: None,
    };

    let serialized = serde_json::to_string(&workload);
    assert!(serialized.is_ok());
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Running;
    let serialized = serde_json::to_string(&status);
    assert!(serialized.is_ok());
}

#[test]
fn test_execution_info_serialization() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Pending,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        runtime_type: None,
        error_message: None,
        output: None,
        metrics: None,
    };

    let serialized = serde_json::to_string(&info);
    assert!(serialized.is_ok());
}

#[test]
fn test_cluster_status_serialization() {
    let status = ClusterStatus {
        total_nodes: 5,
        healthy_nodes: 5,
        cluster_load: 0.5,
        active_executions: 10,
        available_runtimes: vec!["native".to_string()],
    };

    let serialized = serde_json::to_string(&status);
    assert!(serialized.is_ok());
}

// ==================== Conversion Tests ====================

#[test]
fn test_resource_requirements_to_core() {
    let client_resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: Some(10240),
        gpu_required: Some(true),
    };

    let core_resources: toadstool::resources::ResourceRequirements = client_resources.into();
    assert!(core_resources.cpu.min_cores > 0.0);
}

#[test]
fn test_resource_requirements_from_core() {
    let core_resources = toadstool::resources::ResourceRequirements {
        cpu: toadstool::resources::CpuRequirements {
            min_cores: 2.0,
            max_cores: None,
            architecture: None,
        },
        memory: toadstool::resources::MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            max_bytes: None,
        },
        storage: toadstool::resources::StorageRequirements {
            min_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_bytes: None,
            storage_type: None,
        },
        gpu: None,
        network: toadstool::resources::NetworkRequirements::default(),
    };

    let client_resources: ResourceRequirements = core_resources.into();
    assert_eq!(client_resources.cpu_cores, Some(2));
    assert_eq!(client_resources.gpu_required, Some(false));
}

// ==================== Edge Case Tests ====================

#[test]
fn test_empty_args() {
    let workload = WorkloadType::Native {
        executable: "/bin/true".to_string(),
        args: vec![],
        working_dir: None,
    };

    match workload {
        WorkloadType::Native { args, .. } => {
            assert_eq!(args.len(), 0);
        }
        _ => panic!("Expected native workload"),
    }
}

#[test]
fn test_many_args() {
    let args: Vec<String> = (0..1000).map(|i| format!("arg{}", i)).collect();

    let workload = WorkloadType::Native {
        executable: "/bin/echo".to_string(),
        args: args.clone(),
        working_dir: None,
    };

    match workload {
        WorkloadType::Native {
            args: submitted_args,
            ..
        } => {
            assert_eq!(submitted_args.len(), 1000);
        }
        _ => panic!("Expected native workload"),
    }
}

#[test]
fn test_zero_resources() {
    let resources = ResourceRequirements {
        cpu_cores: Some(0),
        memory_mb: Some(0),
        disk_mb: Some(0),
        gpu_required: Some(false),
    };

    assert_eq!(resources.cpu_cores, Some(0));
}

#[test]
fn test_large_resource_values() {
    let resources = ResourceRequirements {
        cpu_cores: Some(128),
        memory_mb: Some(1_048_576), // 1TB
        disk_mb: Some(10_485_760),  // 10TB
        gpu_required: Some(true),
    };

    assert_eq!(resources.cpu_cores, Some(128));
}

// ==================== Clone and Debug Tests ====================

#[test]
fn test_workload_type_clone() {
    let workload = WorkloadType::Native {
        executable: "/bin/ls".to_string(),
        args: vec![],
        working_dir: None,
    };

    let cloned = workload.clone();
    let _ = format!("{:?}", cloned);
}

#[test]
fn test_execution_status_clone() {
    let status = ExecutionStatus::Running;
    let cloned = status.clone();
    let _ = format!("{:?}", cloned);
}

#[test]
fn test_resource_requirements_clone() {
    let resources = ResourceRequirements::default();
    let cloned = resources.clone();
    let _ = format!("{:?}", cloned);
}

// ==================== Equality Tests ====================

#[test]
fn test_resource_requirements_equality() {
    let r1 = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(4096),
        disk_mb: Some(10240),
        gpu_required: Some(false),
    };

    let r2 = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(4096),
        disk_mb: Some(10240),
        gpu_required: Some(false),
    };

    assert_eq!(r1, r2);
}

#[test]
fn test_resource_requirements_inequality() {
    let r1 = ResourceRequirements {
        cpu_cores: Some(2),
        ..Default::default()
    };

    let r2 = ResourceRequirements {
        cpu_cores: Some(4),
        ..Default::default()
    };

    assert_ne!(r1, r2);
}
