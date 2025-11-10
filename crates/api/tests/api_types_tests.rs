//! Comprehensive tests for API types

use chrono::Utc;
use toadstool_api::types::*;
use uuid::Uuid;

// ============================================================================
// ExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_submitted() {
    let status = ExecutionStatus::Submitted;
    assert_eq!(status, ExecutionStatus::Submitted);
}

#[test]
fn test_execution_status_queued() {
    let status = ExecutionStatus::Queued;
    assert_eq!(status, ExecutionStatus::Queued);
}

#[test]
fn test_execution_status_running() {
    let status = ExecutionStatus::Running;
    assert_eq!(status, ExecutionStatus::Running);
}

#[test]
fn test_execution_status_completed() {
    let status = ExecutionStatus::Completed;
    assert_eq!(status, ExecutionStatus::Completed);
}

#[test]
fn test_execution_status_failed() {
    let status = ExecutionStatus::Failed;
    assert_eq!(status, ExecutionStatus::Failed);
}

#[test]
fn test_execution_status_cancelled() {
    let status = ExecutionStatus::Cancelled;
    assert_eq!(status, ExecutionStatus::Cancelled);
}

#[test]
fn test_execution_status_timedout() {
    let status = ExecutionStatus::TimedOut;
    assert_eq!(status, ExecutionStatus::TimedOut);
}

#[test]
fn test_execution_status_paused() {
    let status = ExecutionStatus::Paused;
    assert_eq!(status, ExecutionStatus::Paused);
}

#[test]
fn test_execution_status_clone() {
    let status = ExecutionStatus::Running;
    let cloned = status.clone();
    assert_eq!(status, cloned);
}

// ============================================================================
// WorkloadSpec Tests
// ============================================================================

#[test]
fn test_workload_spec_native() {
    let workload = WorkloadSpec::Native {
        executable: "/usr/bin/python3".to_string(),
        args: vec!["script.py".to_string()],
    };

    if let WorkloadSpec::Native { executable, args } = workload {
        assert_eq!(executable, "/usr/bin/python3");
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected Native variant");
    }
}

#[test]
fn test_workload_spec_container() {
    let workload = WorkloadSpec::Container {
        image: "ubuntu:22.04".to_string(),
        command: Some(vec!["bash".to_string()]),
        args: Some(vec!["-c".to_string(), "echo hello".to_string()]),
    };

    if let WorkloadSpec::Container { image, command, .. } = workload {
        assert_eq!(image, "ubuntu:22.04");
        assert!(command.is_some());
    } else {
        panic!("Expected Container variant");
    }
}

#[test]
fn test_workload_spec_wasm() {
    let workload = WorkloadSpec::Wasm {
        module: "compute.wasm".to_string(),
        function: "calculate".to_string(),
        args: vec!["100".to_string()],
    };

    if let WorkloadSpec::Wasm {
        module, function, ..
    } = workload
    {
        assert_eq!(module, "compute.wasm");
        assert_eq!(function, "calculate");
    } else {
        panic!("Expected Wasm variant");
    }
}

#[test]
fn test_workload_spec_python() {
    let workload = WorkloadSpec::Python {
        script: "import numpy; print('hello')".to_string(),
        requirements: Some(vec!["numpy==1.24.0".to_string()]),
    };

    if let WorkloadSpec::Python {
        script,
        requirements,
    } = workload
    {
        assert!(script.contains("numpy"));
        assert!(requirements.is_some());
    } else {
        panic!("Expected Python variant");
    }
}

#[test]
fn test_workload_spec_gpu() {
    let workload = WorkloadSpec::Gpu {
        kernel: "matrix_multiply".to_string(),
        platform: "cuda".to_string(),
        args: vec!["1024".to_string()],
    };

    if let WorkloadSpec::Gpu {
        kernel, platform, ..
    } = workload
    {
        assert_eq!(kernel, "matrix_multiply");
        assert_eq!(platform, "cuda");
    } else {
        panic!("Expected Gpu variant");
    }
}

// ============================================================================
// LogLevel Tests
// ============================================================================

#[test]
fn test_log_level_trace() {
    let level = LogLevel::Trace;
    assert!(matches!(level, LogLevel::Trace));
}

#[test]
fn test_log_level_debug() {
    let level = LogLevel::Debug;
    assert!(matches!(level, LogLevel::Debug));
}

#[test]
fn test_log_level_info() {
    let level = LogLevel::Info;
    assert!(matches!(level, LogLevel::Info));
}

#[test]
fn test_log_level_warn() {
    let level = LogLevel::Warn;
    assert!(matches!(level, LogLevel::Warn));
}

#[test]
fn test_log_level_error() {
    let level = LogLevel::Error;
    assert!(matches!(level, LogLevel::Error));
}

// ============================================================================
// AlertSeverity Tests
// ============================================================================

#[test]
fn test_alert_severity_info() {
    let severity = AlertSeverity::Info;
    assert!(matches!(severity, AlertSeverity::Info));
}

#[test]
fn test_alert_severity_warning() {
    let severity = AlertSeverity::Warning;
    assert!(matches!(severity, AlertSeverity::Warning));
}

#[test]
fn test_alert_severity_error() {
    let severity = AlertSeverity::Error;
    assert!(matches!(severity, AlertSeverity::Error));
}

#[test]
fn test_alert_severity_critical() {
    let severity = AlertSeverity::Critical;
    assert!(matches!(severity, AlertSeverity::Critical));
}

#[test]
fn test_alert_severity_clone() {
    let severity = AlertSeverity::Critical;
    let cloned = severity.clone();
    assert!(matches!(cloned, AlertSeverity::Critical));
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_minimal() {
    let resources = ResourceRequirements {
        cpu_cores: Some(1.0),
        memory_mb: Some(512),
        storage_mb: Some(1024),
        gpu_count: None,
        network_mbps: None,
    };

    assert_eq!(resources.cpu_cores, Some(1.0));
    assert_eq!(resources.memory_mb, Some(512));
}

#[test]
fn test_resource_requirements_with_gpu() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4.0),
        memory_mb: Some(8192),
        storage_mb: Some(10240),
        gpu_count: Some(2),
        network_mbps: Some(1000),
    };

    assert_eq!(resources.gpu_count, Some(2));
}

#[test]
fn test_resource_requirements_fractional_cpu() {
    let resources = ResourceRequirements {
        cpu_cores: Some(0.5),
        memory_mb: Some(256),
        storage_mb: None,
        gpu_count: None,
        network_mbps: None,
    };

    assert_eq!(resources.cpu_cores, Some(0.5));
}

// ============================================================================
// ResourceAllocation Tests
// ============================================================================

#[test]
fn test_resource_allocation_basic() {
    let allocation = ResourceAllocation {
        node_id: "node-1".to_string(),
        cpu_cores: 2.0,
        memory_mb: 4096,
        storage_mb: 10240,
        gpu_count: 0,
    };

    assert_eq!(allocation.node_id, "node-1");
    assert_eq!(allocation.cpu_cores, 2.0);
}

#[test]
fn test_resource_allocation_with_gpu() {
    let allocation = ResourceAllocation {
        node_id: "gpu-node-3".to_string(),
        cpu_cores: 8.0,
        memory_mb: 32768,
        storage_mb: 102400,
        gpu_count: 4,
    };

    assert_eq!(allocation.gpu_count, 4);
}

// ============================================================================
// MonitoringEndpoints Tests
// ============================================================================

#[test]
fn test_monitoring_endpoints_creation() {
    let endpoints = MonitoringEndpoints {
        status_url: "http://api/status/123".to_string(),
        logs_url: "http://api/logs/123".to_string(),
        metrics_url: "http://api/metrics/123".to_string(),
        websocket_url: "ws://api/ws/123".to_string(),
    };

    assert!(endpoints.status_url.contains("status"));
    assert!(endpoints.websocket_url.starts_with("ws://"));
}

#[test]
fn test_monitoring_endpoints_https() {
    let endpoints = MonitoringEndpoints {
        status_url: "https://secure-api/status".to_string(),
        logs_url: "https://secure-api/logs".to_string(),
        metrics_url: "https://secure-api/metrics".to_string(),
        websocket_url: "wss://secure-api/ws".to_string(),
    };

    assert!(endpoints.status_url.starts_with("https://"));
    assert!(endpoints.websocket_url.starts_with("wss://"));
}

// ============================================================================
// PaginationInfo Tests
// ============================================================================

#[test]
fn test_pagination_info_first_page() {
    let pagination = PaginationInfo {
        page: 1,
        per_page: 20,
        total_pages: 5,
        total_items: 95,
        has_next: true,
        has_prev: false,
    };

    assert_eq!(pagination.page, 1);
    assert!(pagination.has_next);
    assert!(!pagination.has_prev);
}

#[test]
fn test_pagination_info_middle_page() {
    let pagination = PaginationInfo {
        page: 3,
        per_page: 20,
        total_pages: 5,
        total_items: 95,
        has_next: true,
        has_prev: true,
    };

    assert!(pagination.has_next);
    assert!(pagination.has_prev);
}

#[test]
fn test_pagination_info_last_page() {
    let pagination = PaginationInfo {
        page: 5,
        per_page: 20,
        total_pages: 5,
        total_items: 95,
        has_next: false,
        has_prev: true,
    };

    assert!(!pagination.has_next);
    assert!(pagination.has_prev);
}

// ============================================================================
// ExecutionResponse Tests
// ============================================================================

#[test]
fn test_execution_response_submitted() {
    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Submitted,
        submitted_at: Utc::now(),
        estimated_completion: None,
        queue_position: Some(5),
        resource_allocation: None,
        monitoring_endpoints: MonitoringEndpoints {
            status_url: "http://api/status".to_string(),
            logs_url: "http://api/logs".to_string(),
            metrics_url: "http://api/metrics".to_string(),
            websocket_url: "ws://api/ws".to_string(),
        },
    };

    assert_eq!(response.status, ExecutionStatus::Submitted);
    assert_eq!(response.queue_position, Some(5));
}

#[test]
fn test_execution_response_running() {
    let allocation = ResourceAllocation {
        node_id: "node-1".to_string(),
        cpu_cores: 2.0,
        memory_mb: 4096,
        storage_mb: 10240,
        gpu_count: 0,
    };

    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        submitted_at: Utc::now(),
        estimated_completion: Some(Utc::now() + chrono::Duration::minutes(10)),
        queue_position: None,
        resource_allocation: Some(allocation),
        monitoring_endpoints: MonitoringEndpoints {
            status_url: "http://api/status".to_string(),
            logs_url: "http://api/logs".to_string(),
            metrics_url: "http://api/metrics".to_string(),
            websocket_url: "ws://api/ws".to_string(),
        },
    };

    assert_eq!(response.status, ExecutionStatus::Running);
    assert!(response.resource_allocation.is_some());
}

// ============================================================================
// LogEntry Tests
// ============================================================================

#[test]
fn test_log_entry_creation() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        level: LogLevel::Info,
        message: "Application started".to_string(),
        source: "main".to_string(),
    };

    assert!(matches!(entry.level, LogLevel::Info));
    assert_eq!(entry.message, "Application started");
}

#[test]
fn test_log_entry_error() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        level: LogLevel::Error,
        message: "Connection failed".to_string(),
        source: "network".to_string(),
    };

    assert!(matches!(entry.level, LogLevel::Error));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_execution_statuses() {
    let statuses = [
        ExecutionStatus::Submitted,
        ExecutionStatus::Queued,
        ExecutionStatus::Running,
        ExecutionStatus::Completed,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Paused,
    ];

    assert_eq!(statuses.len(), 8);
}

#[test]
fn test_all_workload_specs() {
    let specs = vec![
        WorkloadSpec::Native {
            executable: "app".to_string(),
            args: vec![],
        },
        WorkloadSpec::Container {
            image: "image".to_string(),
            command: None,
            args: None,
        },
        WorkloadSpec::Wasm {
            module: "mod".to_string(),
            function: "fn".to_string(),
            args: vec![],
        },
        WorkloadSpec::Python {
            script: "script".to_string(),
            requirements: None,
        },
        WorkloadSpec::Gpu {
            kernel: "kernel".to_string(),
            platform: "platform".to_string(),
            args: vec![],
        },
    ];

    assert_eq!(specs.len(), 5);
}

#[test]
fn test_all_log_levels() {
    let levels = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    assert_eq!(levels.len(), 5);
}

#[test]
fn test_all_alert_severities() {
    let severities = [
        AlertSeverity::Info,
        AlertSeverity::Warning,
        AlertSeverity::Error,
        AlertSeverity::Critical,
    ];

    assert_eq!(severities.len(), 4);
}
