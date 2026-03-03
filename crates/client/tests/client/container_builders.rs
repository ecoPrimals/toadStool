// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStoolEvent tests - use JSON-RPC polling (no WebSocket)

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool_client::*;
use uuid::Uuid;

#[test]
fn test_event_execution_status_changed() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: Uuid::new_v4().to_string(),
        status: "completed".to_string(),
    };
    match &event {
        ToadStoolEvent::ExecutionStatusChanged { status, .. } => {
            assert_eq!(status, "completed");
        }
        _ => panic!("Expected ExecutionStatusChanged"),
    }
}

#[test]
fn test_event_cluster_health_changed() {
    let event = ToadStoolEvent::ClusterHealthChanged { healthy: true };
    match &event {
        ToadStoolEvent::ClusterHealthChanged { healthy } => assert!(*healthy),
        _ => panic!("Expected ClusterHealthChanged"),
    }
}

#[test]
fn test_event_serialization() {
    let event = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: "id".to_string(),
        status: "running".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_event_clone() {
    let event1 = ToadStoolEvent::ExecutionStatusChanged {
        execution_id: "id-1".to_string(),
        status: "running".to_string(),
    };
    let event2 = event1.clone();
    match (event1, event2) {
        (
            ToadStoolEvent::ExecutionStatusChanged {
                execution_id: id1, ..
            },
            ToadStoolEvent::ExecutionStatusChanged {
                execution_id: id2, ..
            },
        ) => assert_eq!(id1, id2),
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
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
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
    let submitted_time = SystemTime::now();

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
        started_at: Some(SystemTime::now()),
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
        completed_at: Some(SystemTime::now()),
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
