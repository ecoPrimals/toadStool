// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for server state types

use std::time::{Duration, SystemTime};
use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::state::*;
use uuid::Uuid;

// ============================================================================
// ClientInfo Tests
// ============================================================================

#[test]
fn test_client_info_full() {
    let info = ClientInfo {
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        api_key: Some("key123".to_string()),
        authenticated_user: Some("user@example.com".to_string()),
    };

    assert!(info.ip_address.is_some());
    assert!(info.authenticated_user.is_some());
}

#[test]
fn test_client_info_anonymous() {
    let info = ClientInfo {
        ip_address: Some("10.0.0.1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    assert!(info.api_key.is_none());
    assert!(info.authenticated_user.is_none());
}

#[test]
fn test_client_info_clone() {
    let info = ClientInfo {
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: Some("curl/7.68.0".to_string()),
        api_key: None,
        authenticated_user: None,
    };

    let cloned = info.clone();
    assert_eq!(info.ip_address, cloned.ip_address);
}

// ============================================================================
// ActiveExecution Tests
// ============================================================================

#[test]
fn test_active_execution_creation() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Native,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(3600),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    assert_eq!(execution.runtime_type, RuntimeType::Native);
    assert_eq!(execution.status, ExecutionStatus::Running);
}

#[test]
fn test_active_execution_with_timeout() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Container,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(7200), // 2 hours
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("10.0.0.5".to_string()),
            user_agent: Some("app/1.0".to_string()),
            api_key: Some("key456".to_string()),
            authenticated_user: Some("admin@example.com".to_string()),
        },
    };

    assert_eq!(execution.timeout, Duration::from_secs(7200));
}

#[test]
fn test_active_execution_clone() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Wasm,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(1800),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    let cloned = execution.clone();
    assert_eq!(execution.execution_id, cloned.execution_id);
    assert_eq!(execution.runtime_type, cloned.runtime_type);
}

// ============================================================================
// Integration Test
// ============================================================================

#[test]
fn test_execution_with_client_context() {
    let client = ClientInfo {
        ip_address: Some("172.16.0.10".to_string()),
        user_agent: Some("ToadStool-Client/1.0".to_string()),
        api_key: Some("prod-key-789".to_string()),
        authenticated_user: Some("service-account@example.com".to_string()),
    };

    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Container,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(3600),
        status: ExecutionStatus::Running,
        client_info: client,
    };

    assert!(execution.client_info.authenticated_user.is_some());
    assert!(execution.client_info.api_key.is_some());
}
