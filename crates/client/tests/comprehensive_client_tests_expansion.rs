//! Comprehensive client library tests - Expansion Pack
//!
//! This test suite provides additional coverage for WorkloadSubmission,
//! ExecutionInfo, ExecutionOutput, ExecutionMetrics, ToadStoolEvent,
//! ClientError, and integration scenarios.

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use toadstool_client::*;
use uuid::Uuid;

// ============================================================================
// WorkloadSubmission Tests
// ============================================================================

#[test]
fn test_workload_submission_basic() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["test".to_string()],
            working_dir: None,
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert!(matches!(
        submission.workload_type,
        WorkloadType::Native { .. }
    ));
    assert!(submission.priority.is_none());
}

#[test]
fn test_workload_submission_with_priority() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Container {
            image: "nginx:latest".to_string(),
            command: None,
            args: None,
            working_dir: None,
        },
        runtime_hint: None,
        priority: Some(JobPriority::High),
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert_eq!(submission.priority, Some(JobPriority::High));
}

#[test]
fn test_workload_submission_with_timeout() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Python {
            script: "print('hello')".to_string(),
            requirements: vec![],
        },
        runtime_hint: None,
        priority: None,
        timeout: Some(Duration::from_secs(300)),
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert_eq!(submission.timeout, Some(Duration::from_secs(300)));
}

#[test]
fn test_workload_submission_with_environment() {
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());

    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/env".to_string(),
            args: vec![],
            working_dir: None,
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: env.clone(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert_eq!(submission.environment.len(), 2);
    assert_eq!(
        submission.environment.get("KEY1"),
        Some(&"value1".to_string())
    );
}

#[test]
fn test_workload_submission_with_resources() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(4 * 1024 * 1024 * 1024),
        disk_mb: None,
        gpu_required: None,
    };

    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Wasm {
            module_data: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: Some(resources),
        metadata: HashMap::new(),
    };

    assert!(submission.resources.is_some());
    assert_eq!(submission.resources.unwrap().cpu_cores, Some(4));
}

#[test]
fn test_workload_submission_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "alice".to_string());
    metadata.insert("project".to_string(), "demo".to_string());

    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Custom {
            workload_data: serde_json::json!({"type": "test"}),
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: metadata.clone(),
    };

    assert_eq!(submission.metadata.len(), 2);
    assert!(submission.metadata.contains_key("user"));
}

#[test]
fn test_workload_submission_with_runtime_hint() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Container {
            image: "alpine:latest".to_string(),
            command: Some(vec!["sh".to_string()]),
            args: None,
            working_dir: None,
        },
        runtime_hint: Some("docker".to_string()),
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert_eq!(submission.runtime_hint, Some("docker".to_string()));
}

#[test]
fn test_workload_submission_full_configuration() {
    let mut env = HashMap::new();
    env.insert("ENV1".to_string(), "val1".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("meta1".to_string(), "value".to_string());

    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(2048),
        disk_mb: Some(10000),
        gpu_required: Some(true),
    };

    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/usr/bin/test".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
            working_dir: Some("/tmp".to_string()),
        },
        runtime_hint: Some("native".to_string()),
        priority: Some(JobPriority::Critical),
        timeout: Some(Duration::from_secs(600)),
        environment: env.clone(),
        resources: Some(resources),
        metadata: metadata.clone(),
    };

    assert!(submission.runtime_hint.is_some());
    assert_eq!(submission.priority, Some(JobPriority::Critical));
    assert_eq!(submission.timeout, Some(Duration::from_secs(600)));
    assert_eq!(submission.environment.len(), 1);
    assert_eq!(submission.metadata.len(), 1);
}

#[test]
fn test_workload_submission_empty_environment() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Python {
            script: "".to_string(),
            requirements: vec![],
        },
        runtime_hint: None,
        priority: None,
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    assert!(submission.environment.is_empty());
}

#[test]
fn test_workload_submission_clone() {
    let submission1 = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/ls".to_string(),
            args: vec![],
            working_dir: None,
        },
        runtime_hint: None,
        priority: Some(JobPriority::Normal),
        timeout: None,
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    let submission2 = submission1.clone();
    assert_eq!(submission1.priority, submission2.priority);
}

// ============================================================================
// ExecutionInfo Tests
// ============================================================================

#[test]
fn test_execution_info_pending() {
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

    assert!(matches!(info.status, ExecutionStatus::Pending));
    assert!(info.started_at.is_none());
    assert!(info.completed_at.is_none());
}

#[test]
fn test_execution_info_running() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        runtime_type: Some("native".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    assert!(matches!(info.status, ExecutionStatus::Running));
    assert!(info.started_at.is_some());
    assert!(info.completed_at.is_none());
}

#[test]
fn test_execution_info_completed() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        runtime_type: Some("container".to_string()),
        error_message: None,
        output: Some(ExecutionOutput {
            stdout: Some("Success".to_string()),
            stderr: None,
            exit_code: Some(0),
            artifacts: vec![],
        }),
        metrics: Some(ExecutionMetrics {
            duration_ms: 1000,
            cpu_usage_percent: 50.0,
            memory_peak_bytes: 1024 * 1024,
            network_bytes_sent: 100,
            network_bytes_received: 200,
        }),
    };

    assert!(matches!(info.status, ExecutionStatus::Completed));
    assert!(info.started_at.is_some());
    assert!(info.completed_at.is_some());
    assert!(info.output.is_some());
    assert!(info.metrics.is_some());
}

#[test]
fn test_execution_info_failed() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        runtime_type: Some("wasm".to_string()),
        error_message: Some("Runtime error: Out of memory".to_string()),
        output: Some(ExecutionOutput {
            stdout: None,
            stderr: Some("Error: OOM".to_string()),
            exit_code: Some(137),
            artifacts: vec![],
        }),
        metrics: None,
    };

    assert!(matches!(info.status, ExecutionStatus::Failed));
    assert!(info.error_message.is_some());
    assert!(info.error_message.as_ref().unwrap().contains("memory"));
}

#[test]
fn test_execution_info_cancelled() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Cancelled,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: Some(Utc::now()),
        runtime_type: None,
        error_message: Some("Cancelled by user".to_string()),
        output: None,
        metrics: None,
    };

    assert!(matches!(info.status, ExecutionStatus::Cancelled));
    assert!(info.error_message.is_some());
}

#[test]
fn test_execution_info_timeout() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Timeout,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        runtime_type: Some("python".to_string()),
        error_message: Some("Execution exceeded timeout of 300s".to_string()),
        output: None,
        metrics: None,
    };

    assert!(matches!(info.status, ExecutionStatus::Timeout));
    assert!(info.error_message.as_ref().unwrap().contains("timeout"));
}

#[test]
fn test_execution_info_with_runtime_type() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        runtime_type: Some("container-docker".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    assert_eq!(info.runtime_type, Some("container-docker".to_string()));
}

#[test]
fn test_execution_info_serialization() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        runtime_type: Some("native".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("execution_id"));
}

#[test]
fn test_execution_info_clone() {
    let info1 = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        runtime_type: None,
        error_message: None,
        output: None,
        metrics: None,
    };

    let info2 = info1.clone();
    assert_eq!(info1.execution_id, info2.execution_id);
}

// ============================================================================
// ExecutionOutput Tests
// ============================================================================

#[test]
fn test_execution_output_success() {
    let output = ExecutionOutput {
        stdout: Some("Hello, World!".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stdout.is_some());
    assert_eq!(output.exit_code, Some(0));
}

#[test]
fn test_execution_output_with_stderr() {
    let output = ExecutionOutput {
        stdout: Some("Output".to_string()),
        stderr: Some("Warning: deprecated".to_string()),
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stderr.is_some());
    assert!(output.stderr.as_ref().unwrap().contains("Warning"));
}

#[test]
fn test_execution_output_failure() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: Some("Error: file not found".to_string()),
        exit_code: Some(1),
        artifacts: vec![],
    };

    assert!(output.stdout.is_none());
    assert_eq!(output.exit_code, Some(1));
}

#[test]
fn test_execution_output_with_artifacts() {
    let output = ExecutionOutput {
        stdout: Some("Build complete".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![
            "output.txt".to_string(),
            "binary".to_string(),
            "logs.tar.gz".to_string(),
        ],
    };

    assert_eq!(output.artifacts.len(), 3);
    assert!(output.artifacts.contains(&"binary".to_string()));
}

#[test]
fn test_execution_output_large_stdout() {
    let large_output = "x".repeat(10000);
    let output = ExecutionOutput {
        stdout: Some(large_output.clone()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert_eq!(output.stdout.as_ref().unwrap().len(), 10000);
}

#[test]
fn test_execution_output_empty() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    assert!(output.stdout.is_none());
    assert!(output.stderr.is_none());
}

#[test]
fn test_execution_output_no_exit_code() {
    let output = ExecutionOutput {
        stdout: Some("Running...".to_string()),
        stderr: None,
        exit_code: None,
        artifacts: vec![],
    };

    assert!(output.exit_code.is_none());
}

#[test]
fn test_execution_output_signal_terminated() {
    let output = ExecutionOutput {
        stdout: None,
        stderr: Some("Killed".to_string()),
        exit_code: Some(128 + 9), // SIGKILL
        artifacts: vec![],
    };

    assert_eq!(output.exit_code, Some(137));
}

#[test]
fn test_execution_output_serialization() {
    let output = ExecutionOutput {
        stdout: Some("test".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec!["file.txt".to_string()],
    };

    let json = serde_json::to_string(&output).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_execution_output_clone() {
    let output1 = ExecutionOutput {
        stdout: Some("data".to_string()),
        stderr: None,
        exit_code: Some(0),
        artifacts: vec![],
    };

    let output2 = output1.clone();
    assert_eq!(output1.exit_code, output2.exit_code);
}

// ============================================================================
// ExecutionMetrics Tests
// ============================================================================

#[test]
fn test_execution_metrics_basic() {
    let metrics = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 75.5,
        memory_peak_bytes: 512 * 1024 * 1024,
        network_bytes_sent: 1000,
        network_bytes_received: 2000,
    };

    assert_eq!(metrics.duration_ms, 5000);
    assert_eq!(metrics.cpu_usage_percent, 75.5);
}

#[test]
fn test_execution_metrics_short_duration() {
    let metrics = ExecutionMetrics {
        duration_ms: 10,
        cpu_usage_percent: 5.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert!(metrics.duration_ms < 100);
}

#[test]
fn test_execution_metrics_long_duration() {
    let metrics = ExecutionMetrics {
        duration_ms: 3_600_000, // 1 hour
        cpu_usage_percent: 95.0,
        memory_peak_bytes: 16 * 1024 * 1024 * 1024,
        network_bytes_sent: 1024 * 1024 * 1024,
        network_bytes_received: 2 * 1024 * 1024 * 1024,
    };

    assert!(metrics.duration_ms > 1_000_000);
}

#[test]
fn test_execution_metrics_low_cpu() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 0.5,
        memory_peak_bytes: 1024,
        network_bytes_sent: 100,
        network_bytes_received: 100,
    };

    assert!(metrics.cpu_usage_percent < 1.0);
}

#[test]
fn test_execution_metrics_high_cpu() {
    let metrics = ExecutionMetrics {
        duration_ms: 10000,
        cpu_usage_percent: 99.9,
        memory_peak_bytes: 1024 * 1024,
        network_bytes_sent: 1000,
        network_bytes_received: 1000,
    };

    assert!(metrics.cpu_usage_percent > 99.0);
}

#[test]
fn test_execution_metrics_high_memory() {
    let metrics = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 50.0,
        memory_peak_bytes: 32 * 1024 * 1024 * 1024, // 32GB
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert!(metrics.memory_peak_bytes > 16 * 1024 * 1024 * 1024);
}

#[test]
fn test_execution_metrics_high_network() {
    let metrics = ExecutionMetrics {
        duration_ms: 30000,
        cpu_usage_percent: 30.0,
        memory_peak_bytes: 1024 * 1024,
        network_bytes_sent: 10 * 1024 * 1024 * 1024, // 10GB
        network_bytes_received: 5 * 1024 * 1024 * 1024, // 5GB
    };

    assert!(metrics.network_bytes_sent > 1024 * 1024 * 1024);
}

#[test]
fn test_execution_metrics_zero_network() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 10.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 0,
        network_bytes_received: 0,
    };

    assert_eq!(metrics.network_bytes_sent, 0);
    assert_eq!(metrics.network_bytes_received, 0);
}

#[test]
fn test_execution_metrics_serialization() {
    let metrics = ExecutionMetrics {
        duration_ms: 1000,
        cpu_usage_percent: 50.0,
        memory_peak_bytes: 1024,
        network_bytes_sent: 100,
        network_bytes_received: 200,
    };

    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("duration_ms"));
}

#[test]
fn test_execution_metrics_clone() {
    let metrics1 = ExecutionMetrics {
        duration_ms: 5000,
        cpu_usage_percent: 60.0,
        memory_peak_bytes: 2048,
        network_bytes_sent: 500,
        network_bytes_received: 1000,
    };

    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.duration_ms, metrics2.duration_ms);
}

// ============================================================================
// ToadStoolEvent Tests
// ============================================================================

#[test]
fn test_event_execution_started() {
    let event = ToadStoolEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionStarted { execution_id, .. } => {
            assert_ne!(execution_id, Uuid::nil());
        }
        _ => panic!("Expected ExecutionStarted event"),
    }
}

#[test]
fn test_event_execution_completed() {
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
fn test_event_execution_progress() {
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
fn test_event_cluster_event() {
    let event = ToadStoolEvent::ClusterEvent {
        event_type: "node_joined".to_string(),
        node_id: Some("node-123".to_string()),
        message: "New node joined the cluster".to_string(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ClusterEvent {
            event_type,
            node_id,
            ..
        } => {
            assert_eq!(event_type, "node_joined");
            assert!(node_id.is_some());
        }
        _ => panic!("Expected ClusterEvent"),
    }
}

#[test]
fn test_event_alert() {
    let event = ToadStoolEvent::Alert {
        severity: "critical".to_string(),
        message: "Disk space low".to_string(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::Alert {
            severity, message, ..
        } => {
            assert_eq!(severity, "critical");
            assert!(message.contains("Disk"));
        }
        _ => panic!("Expected Alert event"),
    }
}

#[test]
fn test_event_progress_100_percent() {
    let event = ToadStoolEvent::ExecutionProgress {
        execution_id: Uuid::new_v4(),
        progress_percent: 100.0,
        message: Some("Complete".to_string()),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionProgress {
            progress_percent, ..
        } => {
            assert_eq!(progress_percent, 100.0);
        }
        _ => panic!("Expected ExecutionProgress event"),
    }
}

#[test]
fn test_event_progress_no_message() {
    let event = ToadStoolEvent::ExecutionProgress {
        execution_id: Uuid::new_v4(),
        progress_percent: 25.0,
        message: None,
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ExecutionProgress { message, .. } => {
            assert!(message.is_none());
        }
        _ => panic!("Expected ExecutionProgress event"),
    }
}

#[test]
fn test_event_cluster_no_node_id() {
    let event = ToadStoolEvent::ClusterEvent {
        event_type: "maintenance".to_string(),
        node_id: None,
        message: "Cluster maintenance scheduled".to_string(),
        timestamp: Utc::now(),
    };

    match event {
        ToadStoolEvent::ClusterEvent { node_id, .. } => {
            assert!(node_id.is_none());
        }
        _ => panic!("Expected ClusterEvent"),
    }
}

#[test]
fn test_event_serialization() {
    let event = ToadStoolEvent::Alert {
        severity: "warning".to_string(),
        message: "Test alert".to_string(),
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_event_clone() {
    let event1 = ToadStoolEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        timestamp: Utc::now(),
    };

    let event2 = event1.clone();
    match (event1, event2) {
        (
            ToadStoolEvent::ExecutionStarted {
                execution_id: id1, ..
            },
            ToadStoolEvent::ExecutionStarted {
                execution_id: id2, ..
            },
        ) => {
            assert_eq!(id1, id2);
        }
        _ => panic!("Events don't match"),
    }
}

// ============================================================================
// ClientError Tests
// ============================================================================

#[test]
fn test_client_error_authentication() {
    let error = ClientError::Authentication("Invalid API key".to_string());
    assert!(error.to_string().contains("Invalid API key"));
}

#[test]
fn test_client_error_configuration() {
    let error = ClientError::Configuration("Missing base URL".to_string());
    assert!(error.to_string().contains("Missing base URL"));
}

#[test]
fn test_client_error_server() {
    let error = ClientError::Server("Internal server error".to_string());
    assert!(error.to_string().contains("Internal server error"));
}

#[test]
fn test_client_error_timeout() {
    let error = ClientError::Timeout("Request timed out after 30s".to_string());
    assert!(error.to_string().contains("timed out"));
}

#[test]
fn test_client_error_websocket() {
    let error = ClientError::WebSocket("Connection closed".to_string());
    assert!(error.to_string().contains("Connection closed"));
}

#[test]
fn test_client_error_debug() {
    let error = ClientError::Configuration("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(!debug_str.is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_workload_submission_to_execution_info_integration() {
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Native {
            executable: "/bin/test".to_string(),
            args: vec![],
            working_dir: None,
        },
        runtime_hint: Some("native".to_string()),
        priority: Some(JobPriority::High),
        timeout: Some(Duration::from_secs(60)),
        environment: HashMap::new(),
        resources: None,
        metadata: HashMap::new(),
    };

    let execution_info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        runtime_type: submission.runtime_hint.clone(),
        error_message: None,
        output: None,
        metrics: None,
    };

    assert_eq!(execution_info.runtime_type, Some("native".to_string()));
}

#[test]
fn test_execution_lifecycle_integration() {
    let id = Uuid::new_v4();
    let submitted_time = Utc::now();

    // Pending
    let pending = ExecutionInfo {
        execution_id: id,
        status: ExecutionStatus::Pending,
        submitted_at: submitted_time,
        started_at: None,
        completed_at: None,
        runtime_type: None,
        error_message: None,
        output: None,
        metrics: None,
    };
    assert!(matches!(pending.status, ExecutionStatus::Pending));

    // Running
    let running = ExecutionInfo {
        execution_id: id,
        status: ExecutionStatus::Running,
        submitted_at: submitted_time,
        started_at: Some(Utc::now()),
        completed_at: None,
        runtime_type: Some("container".to_string()),
        error_message: None,
        output: None,
        metrics: None,
    };
    assert!(matches!(running.status, ExecutionStatus::Running));

    // Completed
    let completed = ExecutionInfo {
        execution_id: id,
        status: ExecutionStatus::Completed,
        submitted_at: submitted_time,
        started_at: running.started_at,
        completed_at: Some(Utc::now()),
        runtime_type: Some("container".to_string()),
        error_message: None,
        output: Some(ExecutionOutput {
            stdout: Some("Success".to_string()),
            stderr: None,
            exit_code: Some(0),
            artifacts: vec![],
        }),
        metrics: Some(ExecutionMetrics {
            duration_ms: 5000,
            cpu_usage_percent: 50.0,
            memory_peak_bytes: 1024 * 1024,
            network_bytes_sent: 100,
            network_bytes_received: 200,
        }),
    };
    assert!(matches!(completed.status, ExecutionStatus::Completed));
    assert!(completed.output.is_some());
    assert!(completed.metrics.is_some());
}

#[test]
fn test_resource_requirements_full_lifecycle() {
    // Define requirements
    let req = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8 * 1024 * 1024 * 1024),
        disk_mb: Some(100 * 1024 * 1024 * 1024),
        gpu_required: Some(true),
    };

    // Create submission with requirements
    let submission = WorkloadSubmission {
        workload_type: WorkloadType::Container {
            image: "ml-training:latest".to_string(),
            command: None,
            args: None,
            working_dir: None,
        },
        runtime_hint: Some("gpu-container".to_string()),
        priority: Some(JobPriority::Critical),
        timeout: Some(Duration::from_secs(3600)),
        environment: HashMap::new(),
        resources: Some(req.clone()),
        metadata: HashMap::new(),
    };

    // Verify requirements preserved
    assert!(submission.resources.is_some());
    let sub_req = submission.resources.unwrap();
    assert_eq!(sub_req.cpu_cores, req.cpu_cores);
    assert_eq!(sub_req.memory_mb, req.memory_mb);
    assert_eq!(sub_req.gpu_required, req.gpu_required);
}

#[test]
fn test_multiple_priority_levels_integration() {
    let priorities = vec![
        JobPriority::Low,
        JobPriority::Normal,
        JobPriority::High,
        JobPriority::Critical,
        JobPriority::Emergency,
    ];

    for priority in priorities {
        let submission = WorkloadSubmission {
            workload_type: WorkloadType::Python {
                script: "print('test')".to_string(),
                requirements: vec![],
            },
            runtime_hint: None,
            priority: Some(priority),
            timeout: None,
            environment: HashMap::new(),
            resources: None,
            metadata: HashMap::new(),
        };

        assert_eq!(submission.priority, Some(priority));
    }
}
