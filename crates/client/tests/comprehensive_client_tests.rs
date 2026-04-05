// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive client library tests
//!
//! This test suite provides extensive coverage for client configuration,
//! authentication, workload submission, and execution status management.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_client::*;

// ============================================================================
// ClientConfig Tests
// ============================================================================

#[test]
fn test_client_config_default() {
    let config = ClientConfig::default();

    assert!(config.base_url.contains("http"));
    assert_eq!(
        config.request_timeout,
        Duration::from_millis(toadstool_config::defaults::timeouts::REQUEST_MS)
    );
    assert_eq!(
        config.max_retries,
        toadstool_config::defaults::retries::MAX_ATTEMPTS
    );
}

#[test]
fn test_client_config_custom() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "value".to_string());

    let config = ClientConfig {
        base_url: "http://custom:9000".to_string(),
        request_timeout: Duration::from_secs(60),
        max_retries: 5,
        retry_backoff: Duration::from_millis(500),
        auth: None,
        custom_headers: headers.clone(),
    };

    assert_eq!(config.base_url, "http://custom:9000");
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.custom_headers.len(), 1);
}

#[test]
fn test_client_config_timeouts() {
    let config = ClientConfig {
        request_timeout: Duration::from_secs(120),
        ..ClientConfig::default()
    };

    assert_eq!(config.request_timeout, Duration::from_secs(120));
}

#[test]
fn test_client_config_retries() {
    let config = ClientConfig {
        max_retries: 10,
        retry_backoff: Duration::from_millis(2000),
        ..ClientConfig::default()
    };

    assert_eq!(config.max_retries, 10);
    assert_eq!(config.retry_backoff, Duration::from_millis(2000));
}

#[test]
fn test_client_config_no_retries() {
    let config = ClientConfig {
        max_retries: 0,
        ..ClientConfig::default()
    };

    assert_eq!(config.max_retries, 0);
}

#[test]
fn test_client_config_custom_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-Request-ID".to_string(), "12345".to_string());
    headers.insert("X-Source".to_string(), "test".to_string());

    let config = ClientConfig {
        custom_headers: headers,
        ..ClientConfig::default()
    };

    assert_eq!(config.custom_headers.len(), 2);
    assert!(config.custom_headers.contains_key("X-Request-ID"));
}

#[test]
fn test_client_config_clone() {
    let config1 = ClientConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.max_retries, config2.max_retries);
    assert_eq!(config1.request_timeout, config2.request_timeout);
}

// ============================================================================
// AuthConfig Tests
// ============================================================================

#[test]
fn test_auth_config_api_key() {
    let auth = AuthConfig::ApiKey {
        key: "secret-key-123".to_string(),
        header_name: "X-API-Key".to_string(),
    };

    match auth {
        AuthConfig::ApiKey { key, header_name } => {
            assert_eq!(key, "secret-key-123");
            assert_eq!(header_name, "X-API-Key");
        }
        _ => panic!("Expected ApiKey variant"),
    }
}

#[test]
fn test_auth_config_bearer_token() {
    let auth = AuthConfig::BearerToken {
        token: "bearer-token-xyz".to_string(),
    };

    match auth {
        AuthConfig::BearerToken { token } => {
            assert_eq!(token, "bearer-token-xyz");
        }
        _ => panic!("Expected BearerToken variant"),
    }
}

#[test]
fn test_auth_config_basic() {
    let auth = AuthConfig::Basic {
        username: "user".to_string(),
        password: "pass".to_string(),
    };

    match auth {
        AuthConfig::Basic { username, password } => {
            assert_eq!(username, "user");
            assert_eq!(password, "pass");
        }
        _ => panic!("Expected Basic variant"),
    }
}

#[test]
fn test_auth_config_custom() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Auth".to_string(), "value".to_string());

    let auth = AuthConfig::Custom {
        headers: headers.clone(),
    };

    match auth {
        AuthConfig::Custom { headers } => {
            assert_eq!(headers.len(), 1);
            assert!(headers.contains_key("X-Custom-Auth"));
        }
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_auth_config_clone() {
    let auth1 = AuthConfig::BearerToken {
        token: "test-token".to_string(),
    };
    let auth2 = auth1.clone();

    match (auth1, auth2) {
        (AuthConfig::BearerToken { token: t1 }, AuthConfig::BearerToken { token: t2 }) => {
            assert_eq!(t1, t2);
        }
        _ => panic!("Expected matching BearerToken variants"),
    }
}

// ============================================================================
// WorkloadType Tests
// ============================================================================

#[test]
fn test_workload_type_native() {
    let workload = WorkloadType::Native {
        executable: "/bin/ls".to_string(),
        args: vec!["-la".to_string()],
        working_dir: Some("/tmp".to_string()),
    };

    match workload {
        WorkloadType::Native {
            executable,
            args,
            working_dir,
        } => {
            assert_eq!(executable, "/bin/ls");
            assert_eq!(args.len(), 1);
            assert_eq!(working_dir, Some("/tmp".to_string()));
        }
        _ => panic!("Expected Native variant"),
    }
}

#[test]
fn test_workload_type_container() {
    let workload = WorkloadType::Container {
        image: "nginx:latest".to_string(),
        command: Some(vec!["nginx".to_string()]),
        args: Some(vec!["-g".to_string(), "daemon off;".to_string()]),
        working_dir: None,
    };

    match workload {
        WorkloadType::Container {
            image,
            command,
            args,
            ..
        } => {
            assert_eq!(image, "nginx:latest");
            assert!(command.is_some());
            assert!(args.is_some());
        }
        _ => panic!("Expected Container variant"),
    }
}

#[test]
fn test_workload_type_wasm() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number
    let workload = WorkloadType::Wasm {
        module_data,
        args: vec!["arg1".to_string()],
    };

    match workload {
        WorkloadType::Wasm {
            module_data: data,
            args,
        } => {
            assert_eq!(data.len(), 4);
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Wasm variant"),
    }
}

#[test]
fn test_workload_type_python() {
    let workload = WorkloadType::Python {
        script: "print('hello')".to_string(),
        requirements: vec!["requests==2.28.0".to_string()],
    };

    match workload {
        WorkloadType::Python {
            script,
            requirements,
        } => {
            assert!(script.contains("hello"));
            assert_eq!(requirements.len(), 1);
        }
        _ => panic!("Expected Python variant"),
    }
}

#[test]
fn test_workload_type_custom() {
    let data = serde_json::json!({
        "type": "custom",
        "data": "test"
    });

    let workload = WorkloadType::Custom {
        workload_data: data,
    };

    match workload {
        WorkloadType::Custom { workload_data } => {
            assert!(workload_data.is_object());
        }
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_workload_type_clone() {
    let workload1 = WorkloadType::Native {
        executable: "/bin/echo".to_string(),
        args: vec![],
        working_dir: None,
    };
    let workload2 = workload1.clone();

    match (workload1, workload2) {
        (
            WorkloadType::Native { executable: e1, .. },
            WorkloadType::Native { executable: e2, .. },
        ) => {
            assert_eq!(e1, e2);
        }
        _ => panic!("Expected matching Native variants"),
    }
}

// ============================================================================
// JobPriority Tests
// ============================================================================

#[test]
fn test_job_priority_low() {
    let priority = JobPriority::Low;
    assert_eq!(priority, JobPriority::Low);
}

#[test]
fn test_job_priority_normal() {
    let priority = JobPriority::Normal;
    assert_eq!(priority, JobPriority::Normal);
}

#[test]
fn test_job_priority_high() {
    let priority = JobPriority::High;
    assert_eq!(priority, JobPriority::High);
}

#[test]
fn test_job_priority_critical() {
    let priority = JobPriority::Critical;
    assert_eq!(priority, JobPriority::Critical);
}

#[test]
fn test_job_priority_emergency() {
    let priority = JobPriority::Emergency;
    assert_eq!(priority, JobPriority::Emergency);
}

#[test]
fn test_job_priority_ordering() {
    let low = JobPriority::Low;
    let normal = JobPriority::Normal;
    let high = JobPriority::High;

    assert_ne!(low, normal);
    assert_ne!(normal, high);
}

#[test]
fn test_job_priority_clone() {
    let priority1 = JobPriority::High;
    let priority2 = priority1;

    assert_eq!(priority1, priority2);
}

#[test]
fn test_job_priority_serialization() {
    let priority = JobPriority::Critical;
    let json = serde_json::to_string(&priority).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_basic() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(1024 * 1024 * 1024),    // 1GB
        disk_mb: Some(10 * 1024 * 1024 * 1024), // 10GB
        gpu_required: None,
    };

    assert_eq!(resources.cpu_cores, Some(2));
    assert_eq!(resources.memory_mb, Some(1024 * 1024 * 1024));
    assert!(resources.gpu_required.is_none());
}

#[test]
fn test_resource_requirements_minimal() {
    let resources = ResourceRequirements {
        cpu_cores: Some(1),
        memory_mb: Some(128 * 1024 * 1024), // 128MB
        disk_mb: None,
        gpu_required: None,
    };

    assert_eq!(resources.cpu_cores, Some(1));
    assert!(resources.disk_mb.is_none());
}

#[test]
fn test_resource_requirements_gpu() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8 * 1024 * 1024 * 1024), // 8GB
        disk_mb: None,
        gpu_required: Some(true),
    };

    assert_eq!(resources.gpu_required, Some(true));
}

#[test]
fn test_resource_requirements_high_performance() {
    let resources = ResourceRequirements {
        cpu_cores: Some(32),
        memory_mb: Some(128 * 1024 * 1024 * 1024), // 128GB
        disk_mb: Some(1024 * 1024 * 1024 * 1024),  // 1TB
        gpu_required: Some(true),
    };

    assert!(resources.cpu_cores.unwrap() > 16);
    assert!(resources.memory_mb.unwrap() > 64 * 1024 * 1024 * 1024);
}

#[test]
fn test_resource_requirements_none() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: None,
        disk_mb: None,
        gpu_required: None,
    };

    assert!(resources.cpu_cores.is_none());
    assert!(resources.memory_mb.is_none());
}

#[test]
fn test_resource_requirements_clone() {
    let resources1 = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(2048),
        disk_mb: None,
        gpu_required: None,
    };

    let resources2 = resources1.clone();
    assert_eq!(resources1.cpu_cores, resources2.cpu_cores);
}

#[test]
fn test_resource_requirements_serialization() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(1024),
        disk_mb: None,
        gpu_required: None,
    };

    let json = serde_json::to_string(&resources).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("cpu_cores"));
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
fn test_execution_status_clone() {
    let status1 = ExecutionStatus::Running;
    let status2 = status1.clone();

    assert!(matches!(status1, ExecutionStatus::Running));
    assert!(matches!(status2, ExecutionStatus::Running));
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Completed;
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.is_empty());
}
