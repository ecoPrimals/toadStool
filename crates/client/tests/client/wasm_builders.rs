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
