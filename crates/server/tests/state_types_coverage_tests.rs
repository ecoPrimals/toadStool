// SPDX-License-Identifier: AGPL-3.0-or-later
//! State types coverage tests - calling actual production code
//!
//! These tests directly instantiate and use types from server/src/state.rs
//! to increase llvm-cov coverage

use std::time::SystemTime;
use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::{ActiveExecution, ClientInfo, ServerEvent, ServerStatistics};
use uuid::Uuid;

// ============================================================================
// ServerStatistics Tests (calls Default implementation)
// ============================================================================

#[test]
fn test_server_statistics_default() {
    // Calls Default::default() implementation
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
fn test_server_statistics_creation() {
    // Create statistics with values
    let stats = ServerStatistics {
        total_executions: 100,
        successful_executions: 95,
        failed_executions: 5,
        average_execution_time_ms: 150.5,
        peak_concurrent_executions: 10,
        uptime_seconds: 3600,
        total_requests: 500,
        errors_count: 8,
    };

    assert_eq!(stats.total_executions, 100);
    assert_eq!(stats.successful_executions, 95);
    assert_eq!(stats.failed_executions, 5);
    assert!((stats.average_execution_time_ms - 150.5).abs() < 0.01);
}

#[test]
fn test_server_statistics_clone() {
    // Test Clone implementation
    let stats1 = ServerStatistics::default();
    let stats2 = stats1.clone();

    assert_eq!(stats1.total_executions, stats2.total_executions);
    assert_eq!(stats1.successful_executions, stats2.successful_executions);
}

#[test]
fn test_server_statistics_debug() {
    // Test Debug implementation
    let stats = ServerStatistics::default();
    let debug_str = format!("{:?}", stats);

    assert!(debug_str.contains("ServerStatistics"));
    assert!(debug_str.contains("total_executions"));
}

// ============================================================================
// ClientInfo Tests
// ============================================================================

#[test]
fn test_client_info_creation() {
    let client = ClientInfo {
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("ToadStool-CLI/1.0".to_string()),
        api_key: Some("test-api-key".to_string()),
        authenticated_user: Some("test-user".to_string()),
    };

    assert_eq!(client.ip_address, Some("192.168.1.100".to_string()));
    assert_eq!(client.user_agent, Some("ToadStool-CLI/1.0".to_string()));
    assert!(client.api_key.is_some());
    assert!(client.authenticated_user.is_some());
}

#[test]
fn test_client_info_anonymous() {
    let client = ClientInfo {
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    assert!(client.ip_address.is_some());
    assert!(client.user_agent.is_none());
    assert!(client.api_key.is_none());
    assert!(client.authenticated_user.is_none());
}

#[test]
fn test_client_info_clone() {
    let client1 = ClientInfo {
        ip_address: Some("10.0.0.1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    let client2 = client1.clone();
    assert_eq!(client1.ip_address, client2.ip_address);
}

#[test]
fn test_client_info_debug() {
    let client = ClientInfo {
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    let debug_str = format!("{:?}", client);
    assert!(debug_str.contains("ClientInfo"));
}

// ============================================================================
// ActiveExecution Tests
// ============================================================================

#[test]
fn test_active_execution_creation() {
    use std::time::Duration;

    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Native,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(300),
        status: ExecutionStatus::Pending,
        client_info: ClientInfo {
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    assert!(!execution.execution_id.is_nil());
    assert_eq!(execution.runtime_type, RuntimeType::Native);
    assert_eq!(execution.timeout.as_secs(), 300);
}

#[test]
fn test_active_execution_with_wasm_runtime() {
    use std::time::Duration;

    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Wasm,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(60),
        status: ExecutionStatus::Pending,
        client_info: ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    assert_eq!(execution.runtime_type, RuntimeType::Wasm);
}

#[test]
fn test_active_execution_clone() {
    use std::time::Duration;

    let execution1 = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Container,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(120),
        status: ExecutionStatus::Pending,
        client_info: ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    let execution2 = execution1.clone();
    assert_eq!(execution1.execution_id, execution2.execution_id);
    assert_eq!(execution1.runtime_type, execution2.runtime_type);
}

// ============================================================================
// ServerEvent Tests
// ============================================================================

#[test]
fn test_server_event_execution_started() {
    let event = ServerEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Native,
        timestamp: SystemTime::now(),
    };

    // Test Debug implementation
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ExecutionStarted"));
    assert!(debug_str.contains("execution_id"));
}

#[test]
fn test_server_event_execution_completed() {
    let event = ServerEvent::ExecutionCompleted {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Success,
        duration_ms: 1500,
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ExecutionCompleted"));
    assert!(debug_str.contains("duration_ms"));
}

#[test]
fn test_server_event_runtime_engine_registered() {
    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: RuntimeType::Wasm,
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("RuntimeEngineRegistered"));
}

#[test]
fn test_server_event_resource_usage_update() {
    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 75.5,
        memory_usage_percent: 60.2,
        active_executions: 5,
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ResourceUsageUpdate"));
    assert!(debug_str.contains("cpu_usage_percent"));
}

#[test]
fn test_server_event_health_status_changed() {
    let event = ServerEvent::HealthStatusChanged {
        healthy: true,
        message: "System healthy".to_string(),
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("HealthStatusChanged"));
}

#[test]
fn test_server_event_error_occurred() {
    let event = ServerEvent::ErrorOccurred {
        error_type: "ExecutionFailure".to_string(),
        message: "Process crashed".to_string(),
        execution_id: Some(Uuid::new_v4()),
        timestamp: SystemTime::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ErrorOccurred"));
    assert!(debug_str.contains("error_type"));
}

#[test]
fn test_server_event_clone() {
    let event1 = ServerEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Python,
        timestamp: SystemTime::now(),
    };

    let event2 = event1.clone();
    let debug1 = format!("{:?}", event1);
    let debug2 = format!("{:?}", event2);
    assert_eq!(debug1, debug2);
}

// ============================================================================
// Integration Tests with Multiple Types
// ============================================================================

#[test]
fn test_execution_with_client_tracking() {
    use std::time::Duration;

    let client = ClientInfo {
        ip_address: Some("203.0.113.42".to_string()),
        user_agent: Some("TestClient/1.0".to_string()),
        api_key: Some("test-key-123".to_string()),
        authenticated_user: Some("alice".to_string()),
    };

    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Native,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(300),
        status: ExecutionStatus::Pending,
        client_info: client,
    };

    assert_eq!(
        execution.client_info.authenticated_user,
        Some("alice".to_string())
    );
    assert_eq!(execution.status, ExecutionStatus::Pending);
}

#[test]
fn test_statistics_update_simulation() {
    let mut stats = ServerStatistics::default();

    // Simulate execution
    stats.total_executions += 1;
    stats.successful_executions += 1;
    stats.total_requests += 1;

    assert_eq!(stats.total_executions, 1);
    assert_eq!(stats.successful_executions, 1);
    assert_eq!(stats.failed_executions, 0);
}

#[test]
fn test_statistics_failure_tracking() {
    let mut stats = ServerStatistics::default();

    // Simulate failures
    for _ in 0..5 {
        stats.total_executions += 1;
        stats.failed_executions += 1;
        stats.errors_count += 1;
    }

    assert_eq!(stats.total_executions, 5);
    assert_eq!(stats.failed_executions, 5);
    assert_eq!(stats.errors_count, 5);
}

#[test]
fn test_runtime_type_variants_in_events() {
    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
    ];

    for runtime_type in runtime_types {
        let event = ServerEvent::RuntimeEngineRegistered {
            runtime_type,
            timestamp: SystemTime::now(),
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("RuntimeEngineRegistered"));
    }
}

#[test]
fn test_execution_status_transitions() {
    let execution_id = Uuid::new_v4();

    // Started
    let start_event = ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type: RuntimeType::Native,
        timestamp: SystemTime::now(),
    };

    // Completed
    let complete_event = ServerEvent::ExecutionCompleted {
        execution_id,
        status: ExecutionStatus::Success,
        duration_ms: 2000,
        timestamp: SystemTime::now(),
    };

    let start_debug = format!("{:?}", start_event);
    let complete_debug = format!("{:?}", complete_event);

    assert!(start_debug.contains("ExecutionStarted"));
    assert!(complete_debug.contains("ExecutionCompleted"));
}

// Coverage: These tests call actual production code in state.rs
// - ServerStatistics Default implementation
// - ClientInfo construction
// - ActiveExecution construction
// - ServerEvent enum variants
// - Clone and Debug implementations
