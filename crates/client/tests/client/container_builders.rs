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
