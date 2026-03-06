// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for API types

use super::*;
use std::time::SystemTime;
use uuid::Uuid;
use validator::Validate;

// ========================================================================
// ExecutionStatus Tests
// ========================================================================

#[test]
fn test_execution_status_variants() {
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
        // Should serialize and deserialize
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"running\"");
}

// ========================================================================
// WorkloadSpec Tests
// ========================================================================

#[test]
fn test_workload_spec_native() {
    let spec = WorkloadSpec::Native {
        executable: "/bin/echo".to_string(),
        args: vec!["hello".to_string()],
    };

    let json = serde_json::to_string(&spec).unwrap();
    let deserialized: WorkloadSpec = serde_json::from_str(&json).unwrap();

    match deserialized {
        WorkloadSpec::Native { executable, args } => {
            assert_eq!(executable, "/bin/echo");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_workload_spec_container() {
    let spec = WorkloadSpec::Container {
        image: "nginx:latest".to_string(),
        command: Some(vec!["nginx".to_string()]),
        args: Some(vec!["-g".to_string(), "daemon off;".to_string()]),
    };

    assert!(serde_json::to_string(&spec).is_ok());
}

#[test]
fn test_workload_spec_wasm() {
    let spec = WorkloadSpec::Wasm {
        module: "module.wasm".to_string(),
        function: "main".to_string(),
        args: vec![],
    };

    assert!(serde_json::to_string(&spec).is_ok());
}

#[test]
fn test_workload_spec_python() {
    let spec = WorkloadSpec::Python {
        script: "print('hello')".to_string(),
        requirements: Some(vec!["numpy".to_string()]),
    };

    assert!(serde_json::to_string(&spec).is_ok());
}

#[test]
fn test_workload_spec_gpu() {
    let spec = WorkloadSpec::Gpu {
        kernel: "matmul.cl".to_string(),
        platform: "opencl".to_string(),
        args: vec!["1024".to_string()],
    };

    assert!(serde_json::to_string(&spec).is_ok());
}

// ========================================================================
// ExecutionRequest Validation Tests
// ========================================================================

#[test]
fn test_execution_request_validation_success() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["test".to_string()],
        },
        runtime_type: RuntimeType::Native,
        priority: 5,
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_execution_request_validation_invalid_priority() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: "/bin/echo".to_string(),
            args: vec![],
        },
        runtime_type: RuntimeType::Native,
        priority: 11, // Invalid: > 10
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };

    assert!(request.validate().is_err());
}

#[test]
fn test_execution_request_validation_empty_executable() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: String::new(), // Invalid: empty
            args: vec![],
        },
        runtime_type: RuntimeType::Native,
        priority: 5,
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };

    assert!(request.validate().is_err());
}

#[test]
fn test_execution_request_validation_empty_image() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Container {
            image: String::new(), // Invalid: empty
            command: None,
            args: None,
        },
        runtime_type: RuntimeType::Container,
        priority: 5,
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };

    assert!(request.validate().is_err());
}

// ========================================================================
// ResourceRequirements Tests
// ========================================================================

#[test]
fn test_resource_requirements_validation() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4.0),
        memory_mb: Some(8192),
        storage_mb: Some(10240),
        gpu_count: Some(1),
        network_mbps: Some(1000),
    };

    assert!(resources.validate().is_ok());
}

#[test]
fn test_resource_requirements_invalid_cpu() {
    let resources = ResourceRequirements {
        cpu_cores: Some(0.05), // Invalid: < 0.1
        memory_mb: Some(1024),
        storage_mb: Some(1024),
        gpu_count: None,
        network_mbps: None,
    };

    assert!(resources.validate().is_err());
}

#[test]
fn test_resource_requirements_invalid_memory() {
    let resources = ResourceRequirements {
        cpu_cores: Some(1.0),
        memory_mb: Some(0), // Invalid: < 1
        storage_mb: Some(1024),
        gpu_count: None,
        network_mbps: None,
    };

    assert!(resources.validate().is_err());
}

// ========================================================================
// ExecutionResponse Tests
// ========================================================================

#[test]
fn test_execution_response_creation() {
    let response = ExecutionResponse {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Queued,
        submitted_at: SystemTime::now(),
        estimated_completion: Some(SystemTime::now()),
        queue_position: Some(5),
        resource_allocation: None,
        monitoring_endpoints: MonitoringEndpoints {
            status_url: "http://localhost/status".to_string(),
            logs_url: "http://localhost/logs".to_string(),
            metrics_url: "http://localhost/metrics".to_string(),
            events_poll_url: "http://localhost/jsonrpc".to_string(),
        },
    };

    assert!(serde_json::to_string(&response).is_ok());
}

// ========================================================================
// NodeStatus Tests
// ========================================================================

#[test]
fn test_node_status_variants() {
    let statuses = vec![
        NodeStatus::Healthy,
        NodeStatus::Degraded,
        NodeStatus::Unhealthy,
        NodeStatus::Offline,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: NodeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}

// ========================================================================
// ApiError Tests
// ========================================================================

#[test]
fn test_api_error_new() {
    let error = ApiError::new("TEST_ERROR", "Test message");

    assert_eq!(error.error_code, "TEST_ERROR");
    assert_eq!(error.message, "Test message");
    assert!(error.details.is_none());
    assert!(error.request_id.is_none());
    assert!(error.documentation_url.is_some());
}

#[test]
fn test_api_error_with_details() {
    let error = ApiError::new("TEST_ERROR", "Test message")
        .with_details(serde_json::json!({"field": "value"}));

    assert!(error.details.is_some());
}

#[test]
fn test_api_error_with_request_id() {
    let error = ApiError::new("TEST_ERROR", "Test message").with_request_id("req-123".to_string());

    assert_eq!(error.request_id, Some("req-123".to_string()));
}

#[test]
fn test_api_error_from_toadstool_error() {
    let ts_error = ToadStoolError::execution("Test execution error");
    let api_error = ApiError::from_toadstool_error(ts_error);

    assert_eq!(api_error.error_code, "EXECUTION_ERROR");
}

#[test]
fn test_api_error_into_response() {
    let error = ApiError::new("NOT_FOUND", "Resource not found");
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ========================================================================
// ExecutionFilter Tests
// ========================================================================

#[test]
fn test_execution_filter_default() {
    let filter = ExecutionFilter::default();

    assert_eq!(filter.page, Some(1));
    assert_eq!(filter.per_page, Some(20));
    assert!(filter.status.is_none());
    assert!(filter.runtime_type.is_none());
}

#[test]
fn test_execution_filter_validation() {
    let filter = ExecutionFilter {
        status: Some(ExecutionStatus::Running),
        runtime_type: Some(RuntimeType::Native),
        submitted_after: Some(SystemTime::now()),
        submitted_before: Some(SystemTime::now()),
        page: Some(1),
        per_page: Some(50),
    };

    assert!(filter.validate().is_ok());
}

#[test]
fn test_execution_filter_invalid_page() {
    let filter = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(0), // Invalid: < 1
        per_page: Some(20),
    };

    assert!(filter.validate().is_err());
}

#[test]
fn test_execution_filter_invalid_per_page() {
    let filter = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(101), // Invalid: > 100
    };

    assert!(filter.validate().is_err());
}

// ========================================================================
// ApiMetrics Tests
// ========================================================================

#[test]
fn test_api_metrics_default() {
    let metrics = ApiMetrics::default();

    assert_eq!(metrics.total_requests, 0);
    assert_eq!(metrics.successful_requests, 0);
    assert_eq!(metrics.failed_requests, 0);
    assert_eq!(metrics.average_response_time_ms, 0.0);
    assert_eq!(metrics.active_connections, 0);
    assert_eq!(metrics.uptime_seconds, 0);
}

#[test]
fn test_api_metrics_serialization() {
    let metrics = ApiMetrics {
        total_requests: 1000,
        successful_requests: 950,
        failed_requests: 50,
        average_response_time_ms: 125.5,
        active_connections: 10,
        uptime_seconds: 3600,
    };

    let json = serde_json::to_string(&metrics).unwrap();
    let deserialized: ApiMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.total_requests, 1000);
    assert_eq!(deserialized.average_response_time_ms, 125.5);
}

// ========================================================================
// LogLevel Tests
// ========================================================================

#[test]
fn test_log_level_variants() {
    let levels = vec![
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for level in levels {
        assert!(serde_json::to_string(&level).is_ok());
    }
}

// ========================================================================
// AlertSeverity Tests
// ========================================================================

#[test]
fn test_alert_severity_variants() {
    let severities = vec![
        AlertSeverity::Info,
        AlertSeverity::Warning,
        AlertSeverity::Error,
        AlertSeverity::Critical,
    ];

    for severity in severities {
        assert!(serde_json::to_string(&severity).is_ok());
    }
}

// ========================================================================
// PaginationInfo Tests
// ========================================================================

#[test]
fn test_pagination_info() {
    let pagination = PaginationInfo {
        page: 2,
        per_page: 20,
        total_pages: 10,
        total_items: 195,
        has_next: true,
        has_prev: true,
    };

    let json = serde_json::to_string(&pagination).unwrap();
    let deserialized: PaginationInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.page, 2);
    assert_eq!(deserialized.total_pages, 10);
    assert!(deserialized.has_next);
    assert!(deserialized.has_prev);
}

// ========================================================================
// AuthRequest Validation Tests
// ========================================================================

#[test]
fn test_auth_request_validation() {
    let request = AuthRequest {
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_auth_request_empty_username() {
    let request = AuthRequest {
        username: String::new(),
        password: "password123".to_string(),
    };

    assert!(request.validate().is_err());
}

#[test]
fn test_auth_request_empty_password() {
    let request = AuthRequest {
        username: "testuser".to_string(),
        password: String::new(),
    };

    assert!(request.validate().is_err());
}

// ========================================================================
// ApiConfig Tests
// ========================================================================

#[test]
fn test_api_config_default() {
    let config = ApiConfig::default();

    assert!(config.enable_rest);
    assert!(config.cors_enabled);
    assert_eq!(config.api_version, "2.0.0");
    assert!(config.enable_metrics);
    assert!(config.enable_tracing);
}

#[test]
fn test_api_config_serialization() {
    let config = ApiConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ApiConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.api_version, config.api_version);
}

// ========================================================================
// ClusterCapacity Tests
// ========================================================================

#[test]
fn test_cluster_capacity_serialization() {
    let capacity = ClusterCapacity {
        cpu_cores: 128,
        memory_gb: 512,
        storage_gb: 10240,
        gpu_count: 8,
    };

    let json = serde_json::to_string(&capacity).unwrap();
    let deserialized: ClusterCapacity = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cpu_cores, 128);
    assert_eq!(deserialized.gpu_count, 8);
}

// ========================================================================
// ResourceUsage Tests
// ========================================================================

#[test]
fn test_resource_usage_serialization() {
    let usage = ResourceUsage {
        cpu_percent: 75.5,
        memory_bytes: 1073741824,   // 1GB
        disk_bytes: 10737418240,    // 10GB
        network_bytes_in: 1048576,  // 1MB
        network_bytes_out: 2097152, // 2MB
        gpu_percent: Some(80.0),
    };

    let json = serde_json::to_string(&usage).unwrap();
    let deserialized: ResourceUsage = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cpu_percent, 75.5);
    assert_eq!(deserialized.gpu_percent, Some(80.0));
}

// ========================================================================
// TimeRange Tests
// ========================================================================

#[test]
fn test_time_range_serialization() {
    let start = SystemTime::now();
    let end = start
        .checked_add(std::time::Duration::from_secs(3600))
        .unwrap_or(start);

    let time_range = TimeRange { start, end };

    let json = serde_json::to_string(&time_range).unwrap();
    let deserialized: TimeRange = serde_json::from_str(&json).unwrap();

    assert!(deserialized.end > deserialized.start);
}

// ========================================================================
// ExecutionInfo Tests
// ========================================================================

#[test]
fn test_api_error_validation_error() {
    use validator::ValidationErrors;

    let mut errors = ValidationErrors::new();
    errors.add("field", validator::ValidationError::new("required"));
    let api_err = ApiError::validation_error(&errors);
    assert_eq!(api_err.error_code, "VALIDATION_ERROR");
    assert!(api_err.details.is_some());
}

#[test]
fn test_api_error_into_response_status_codes() {
    let codes = [
        ("VALIDATION_ERROR", StatusCode::BAD_REQUEST),
        ("NOT_FOUND", StatusCode::NOT_FOUND),
        ("UNAUTHORIZED", StatusCode::UNAUTHORIZED),
        ("FORBIDDEN", StatusCode::FORBIDDEN),
        ("RATE_LIMITED", StatusCode::TOO_MANY_REQUESTS),
        ("TIMEOUT", StatusCode::REQUEST_TIMEOUT),
    ];
    for (code, expected) in codes {
        let err = ApiError::new(code, "test");
        let resp = err.into_response();
        assert_eq!(
            resp.status(),
            expected,
            "code {} should map to {}",
            code,
            expected
        );
    }
}

#[test]
fn test_execution_request_validation_empty_wasm() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Wasm {
            module: String::new(),
            function: "main".to_string(),
            args: vec![],
        },
        runtime_type: RuntimeType::Wasm,
        priority: 5,
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };
    assert!(request.validate().is_err());
}

#[test]
fn test_execution_request_validation_empty_gpu() {
    let request = ExecutionRequest {
        workload: WorkloadSpec::Gpu {
            kernel: String::new(),
            platform: "opencl".to_string(),
            args: vec![],
        },
        runtime_type: RuntimeType::Gpu,
        priority: 5,
        timeout_secs: Some(60),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    };
    assert!(request.validate().is_err());
}

#[test]
fn test_execution_info_clone() {
    let info = ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Native,
        submitted_at: SystemTime::now(),
        started_at: Some(SystemTime::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(0.5),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    let cloned = info.clone();
    assert_eq!(info.execution_id, cloned.execution_id);
    assert_eq!(info.status, cloned.status);
}
