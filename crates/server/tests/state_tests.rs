// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for server state types

use std::time::{Duration, SystemTime};
use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::*;
use uuid::Uuid;

// ============================================================================
// ServerEvent Tests
// ============================================================================

#[test]
fn test_server_event_execution_started_creation() {
    let execution_id = Uuid::new_v4();
    let runtime_type = RuntimeType::Native;
    let timestamp = SystemTime::now();

    let event = ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type,
        timestamp,
    };

    match event {
        ServerEvent::ExecutionStarted {
            execution_id: id,
            runtime_type: rt,
            ..
        } => {
            assert_eq!(id, execution_id);
            assert_eq!(rt, RuntimeType::Native);
        }
        _ => panic!("Expected ExecutionStarted event"),
    }
}

#[test]
fn test_server_event_execution_completed_creation() {
    let execution_id = Uuid::new_v4();
    let status = ExecutionStatus::Success;
    let duration_ms = 1500u64;
    let timestamp = SystemTime::now();

    let event = ServerEvent::ExecutionCompleted {
        execution_id,
        status: status.clone(),
        duration_ms,
        timestamp,
    };

    match event {
        ServerEvent::ExecutionCompleted {
            execution_id: id,
            status: s,
            duration_ms: d,
            ..
        } => {
            assert_eq!(id, execution_id);
            assert_eq!(s, status);
            assert_eq!(d, 1500);
        }
        _ => panic!("Expected ExecutionCompleted event"),
    }
}

#[test]
fn test_server_event_runtime_engine_registered_creation() {
    let runtime_type = RuntimeType::Wasm;
    let timestamp = SystemTime::now();

    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type,
        timestamp,
    };

    match event {
        ServerEvent::RuntimeEngineRegistered {
            runtime_type: rt, ..
        } => {
            assert_eq!(rt, RuntimeType::Wasm);
        }
        _ => panic!("Expected RuntimeEngineRegistered event"),
    }
}

#[test]
fn test_server_event_resource_usage_update_creation() {
    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 45.5,
        memory_usage_percent: 67.3,
        active_executions: 12,
        timestamp: SystemTime::now(),
    };

    match event {
        ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent,
            memory_usage_percent,
            active_executions,
            ..
        } => {
            assert_eq!(cpu_usage_percent, 45.5);
            assert_eq!(memory_usage_percent, 67.3);
            assert_eq!(active_executions, 12);
        }
        _ => panic!("Expected ResourceUsageUpdate event"),
    }
}

#[test]
fn test_server_event_health_status_changed_creation() {
    let event = ServerEvent::HealthStatusChanged {
        healthy: true,
        message: "All systems operational".to_string(),
        timestamp: SystemTime::now(),
    };

    match event {
        ServerEvent::HealthStatusChanged {
            healthy, message, ..
        } => {
            assert!(healthy);
            assert_eq!(message, "All systems operational");
        }
        _ => panic!("Expected HealthStatusChanged event"),
    }
}

#[test]
fn test_server_event_error_occurred_creation() {
    let execution_id = Uuid::new_v4();
    let event = ServerEvent::ErrorOccurred {
        error_type: "RuntimeError".to_string(),
        message: "Execution timeout".to_string(),
        execution_id: Some(execution_id),
        timestamp: SystemTime::now(),
    };

    match event {
        ServerEvent::ErrorOccurred {
            error_type,
            message,
            execution_id: id,
            ..
        } => {
            assert_eq!(error_type, "RuntimeError");
            assert_eq!(message, "Execution timeout");
            assert_eq!(id, Some(execution_id));
        }
        _ => panic!("Expected ErrorOccurred event"),
    }
}

#[test]
fn test_server_event_error_occurred_without_execution_id() {
    let event = ServerEvent::ErrorOccurred {
        error_type: "ConfigError".to_string(),
        message: "Invalid configuration".to_string(),
        execution_id: None,
        timestamp: SystemTime::now(),
    };

    match event {
        ServerEvent::ErrorOccurred {
            execution_id: id, ..
        } => {
            assert!(id.is_none());
        }
        _ => panic!("Expected ErrorOccurred event"),
    }
}

#[test]
fn test_server_event_clone() {
    let event = ServerEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Container,
        timestamp: SystemTime::now(),
    };

    let cloned = event.clone();

    match (event, cloned) {
        (
            ServerEvent::ExecutionStarted {
                execution_id: id1, ..
            },
            ServerEvent::ExecutionStarted {
                execution_id: id2, ..
            },
        ) => {
            assert_eq!(id1, id2);
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_server_event_debug_format() {
    let event = ServerEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Python,
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ExecutionStarted"));
    assert!(debug_str.contains("Python"));
}

// ============================================================================
// ActiveExecution Tests
// ============================================================================

#[test]
fn test_active_execution_creation() {
    let execution_id = Uuid::new_v4();
    let runtime_type = RuntimeType::Native;
    let started_at = SystemTime::now();
    let timeout = Duration::from_secs(300);
    let status = ExecutionStatus::Running;

    let client_info = ClientInfo {
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("ToadStool-Client/1.0".to_string()),
        api_key: Some("test-api-key".to_string()),
        authenticated_user: Some("user@example.com".to_string()),
    };

    let execution = ActiveExecution {
        execution_id,
        runtime_type,
        started_at,
        timeout,
        status: status.clone(),
        client_info: client_info.clone(),
    };

    assert_eq!(execution.execution_id, execution_id);
    assert_eq!(execution.runtime_type, RuntimeType::Native);
    assert_eq!(execution.status, status);
    assert_eq!(execution.timeout, Duration::from_secs(300));
}

#[test]
fn test_active_execution_clone() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Gpu,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(600),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    let cloned = execution.clone();

    assert_eq!(execution.execution_id, cloned.execution_id);
    assert_eq!(execution.runtime_type, cloned.runtime_type);
    assert_eq!(execution.timeout, cloned.timeout);
}

#[test]
fn test_active_execution_debug_format() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Wasm,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(120),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    let debug_str = format!("{:?}", execution);
    assert!(debug_str.contains("ActiveExecution"));
    assert!(debug_str.contains("Wasm"));
}

#[test]
fn test_active_execution_with_different_timeouts() {
    let timeouts = vec![
        Duration::from_secs(30),
        Duration::from_secs(300),
        Duration::from_secs(3600),
        Duration::from_secs(86400),
    ];

    for timeout in timeouts {
        let execution = ActiveExecution {
            execution_id: Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            started_at: SystemTime::now(),
            timeout,
            status: ExecutionStatus::Running,
            client_info: ClientInfo {
                ip_address: None,
                user_agent: None,
                api_key: None,
                authenticated_user: None,
            },
        };

        assert_eq!(execution.timeout, timeout);
    }
}

#[test]
fn test_active_execution_with_all_runtime_types() {
    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
        RuntimeType::Gpu,
    ];

    for rt in runtime_types {
        let execution = ActiveExecution {
            execution_id: Uuid::new_v4(),
            runtime_type: rt.clone(),
            started_at: SystemTime::now(),
            timeout: Duration::from_secs(300),
            status: ExecutionStatus::Running,
            client_info: ClientInfo {
                ip_address: None,
                user_agent: None,
                api_key: None,
                authenticated_user: None,
            },
        };

        assert_eq!(execution.runtime_type, rt);
    }
}

#[test]
fn test_active_execution_with_all_execution_statuses() {
    let statuses = vec![
        ExecutionStatus::Running,
        ExecutionStatus::Success,
        ExecutionStatus::Failed {
            error: "test error".to_string(),
        },
        ExecutionStatus::Cancelled,
    ];

    for status in statuses {
        let execution = ActiveExecution {
            execution_id: Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            started_at: SystemTime::now(),
            timeout: Duration::from_secs(300),
            status: status.clone(),
            client_info: ClientInfo {
                ip_address: None,
                user_agent: None,
                api_key: None,
                authenticated_user: None,
            },
        };

        assert_eq!(execution.status, status);
    }
}

// ============================================================================
// ClientInfo Tests
// ============================================================================

#[test]
fn test_client_info_creation_with_all_fields() {
    let client_info = ClientInfo {
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("ToadStool-CLI/1.0".to_string()),
        api_key: Some("sk-test-12345".to_string()),
        authenticated_user: Some("admin@example.com".to_string()),
    };

    assert_eq!(client_info.ip_address, Some("192.168.1.100".to_string()));
    assert_eq!(
        client_info.user_agent,
        Some("ToadStool-CLI/1.0".to_string())
    );
    assert_eq!(client_info.api_key, Some("sk-test-12345".to_string()));
    assert_eq!(
        client_info.authenticated_user,
        Some("admin@example.com".to_string())
    );
}

#[test]
fn test_client_info_creation_with_no_fields() {
    let client_info = ClientInfo {
        ip_address: None,
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    assert!(client_info.ip_address.is_none());
    assert!(client_info.user_agent.is_none());
    assert!(client_info.api_key.is_none());
    assert!(client_info.authenticated_user.is_none());
}

#[test]
fn test_client_info_with_partial_fields() {
    let client_info = ClientInfo {
        ip_address: Some("10.0.0.50".to_string()),
        user_agent: Some("curl/7.68.0".to_string()),
        api_key: None,
        authenticated_user: None,
    };

    assert!(client_info.ip_address.is_some());
    assert!(client_info.user_agent.is_some());
    assert!(client_info.api_key.is_none());
    assert!(client_info.authenticated_user.is_none());
}

#[test]
fn test_client_info_clone() {
    let client_info = ClientInfo {
        ip_address: Some("172.16.0.1".to_string()),
        user_agent: Some("Test-Agent".to_string()),
        api_key: Some("test-key".to_string()),
        authenticated_user: Some("test-user".to_string()),
    };

    let cloned = client_info.clone();

    assert_eq!(client_info.ip_address, cloned.ip_address);
    assert_eq!(client_info.user_agent, cloned.user_agent);
    assert_eq!(client_info.api_key, cloned.api_key);
    assert_eq!(client_info.authenticated_user, cloned.authenticated_user);
}

#[test]
fn test_client_info_debug_format() {
    let client_info = ClientInfo {
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: Some("Test".to_string()),
        api_key: Some("key".to_string()),
        authenticated_user: Some("user".to_string()),
    };

    let debug_str = format!("{:?}", client_info);
    assert!(debug_str.contains("ClientInfo"));
}

#[test]
fn test_client_info_with_ipv6_address() {
    let client_info = ClientInfo {
        ip_address: Some("::1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    assert_eq!(client_info.ip_address, Some("::1".to_string()));
}

#[test]
fn test_client_info_with_long_user_agent() {
    let long_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string();
    let client_info = ClientInfo {
        ip_address: None,
        user_agent: Some(long_ua.clone()),
        api_key: None,
        authenticated_user: None,
    };

    assert_eq!(client_info.user_agent, Some(long_ua));
}

// ============================================================================
// ServerStatistics Tests
// ============================================================================

#[test]
fn test_server_statistics_default() {
    let stats = ServerStatistics::default();

    assert_eq!(stats.total_executions, 0);
    assert_eq!(stats.successful_executions, 0);
    assert_eq!(stats.failed_executions, 0);
    assert_eq!(stats.average_execution_time_ms, 0.0);
    assert_eq!(stats.peak_concurrent_executions, 0);
    assert_eq!(stats.uptime_seconds, 0);
    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.errors_count, 0);
}

#[test]
fn test_server_statistics_creation_with_values() {
    let stats = ServerStatistics {
        total_executions: 1000,
        successful_executions: 950,
        failed_executions: 50,
        average_execution_time_ms: 1234.56,
        peak_concurrent_executions: 25,
        uptime_seconds: 86400,
        total_requests: 5000,
        errors_count: 75,
    };

    assert_eq!(stats.total_executions, 1000);
    assert_eq!(stats.successful_executions, 950);
    assert_eq!(stats.failed_executions, 50);
    assert_eq!(stats.average_execution_time_ms, 1234.56);
    assert_eq!(stats.peak_concurrent_executions, 25);
    assert_eq!(stats.uptime_seconds, 86400);
    assert_eq!(stats.total_requests, 5000);
    assert_eq!(stats.errors_count, 75);
}

#[test]
fn test_server_statistics_clone() {
    let stats = ServerStatistics {
        total_executions: 100,
        successful_executions: 90,
        failed_executions: 10,
        average_execution_time_ms: 500.0,
        peak_concurrent_executions: 10,
        uptime_seconds: 3600,
        total_requests: 200,
        errors_count: 15,
    };

    let cloned = stats.clone();

    assert_eq!(stats.total_executions, cloned.total_executions);
    assert_eq!(stats.successful_executions, cloned.successful_executions);
    assert_eq!(stats.failed_executions, cloned.failed_executions);
    assert_eq!(
        stats.average_execution_time_ms,
        cloned.average_execution_time_ms
    );
}

#[test]
fn test_server_statistics_debug_format() {
    let stats = ServerStatistics::default();
    let debug_str = format!("{:?}", stats);

    assert!(debug_str.contains("ServerStatistics"));
}

#[test]
fn test_server_statistics_with_high_values() {
    let stats = ServerStatistics {
        total_executions: u64::MAX,
        successful_executions: u64::MAX - 1,
        failed_executions: 1,
        average_execution_time_ms: f64::MAX,
        peak_concurrent_executions: u32::MAX,
        uptime_seconds: u64::MAX,
        total_requests: u64::MAX,
        errors_count: u64::MAX,
    };

    assert_eq!(stats.total_executions, u64::MAX);
    assert_eq!(stats.peak_concurrent_executions, u32::MAX);
}

#[test]
fn test_server_statistics_success_rate_calculation() {
    let stats = ServerStatistics {
        total_executions: 1000,
        successful_executions: 980,
        failed_executions: 20,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 0,
        total_requests: 0,
        errors_count: 0,
    };

    let success_rate = (stats.successful_executions as f64 / stats.total_executions as f64) * 100.0;
    assert_eq!(success_rate, 98.0);
}

#[test]
fn test_server_statistics_failure_rate_calculation() {
    let stats = ServerStatistics {
        total_executions: 500,
        successful_executions: 450,
        failed_executions: 50,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 0,
        total_requests: 0,
        errors_count: 0,
    };

    let failure_rate = (stats.failed_executions as f64 / stats.total_executions as f64) * 100.0;
    assert_eq!(failure_rate, 10.0);
}

#[test]
fn test_server_statistics_with_zero_executions() {
    let stats = ServerStatistics {
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 3600,
        total_requests: 100,
        errors_count: 5,
    };

    assert_eq!(stats.total_executions, 0);
    assert_eq!(stats.uptime_seconds, 3600);
    assert_eq!(stats.total_requests, 100);
}

#[test]
fn test_server_statistics_uptime_days_calculation() {
    let stats = ServerStatistics {
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 86400 * 7, // 7 days
        total_requests: 0,
        errors_count: 0,
    };

    let uptime_days = stats.uptime_seconds / 86400;
    assert_eq!(uptime_days, 7);
}

#[test]
fn test_server_statistics_requests_per_second() {
    let stats = ServerStatistics {
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 3600, // 1 hour
        total_requests: 7200,
        errors_count: 0,
    };

    let rps = stats.total_requests as f64 / stats.uptime_seconds as f64;
    assert_eq!(rps, 2.0); // 2 requests per second
}

#[test]
fn test_server_statistics_error_rate() {
    let stats = ServerStatistics {
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0.0,
        peak_concurrent_executions: 0,
        uptime_seconds: 0,
        total_requests: 1000,
        errors_count: 25,
    };

    let error_rate = (stats.errors_count as f64 / stats.total_requests as f64) * 100.0;
    assert_eq!(error_rate, 2.5);
}
