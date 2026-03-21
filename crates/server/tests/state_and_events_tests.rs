// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Tests for server state and event types
//!
//! Week 13 Day 1: State Management and Event Handling Tests
//! Target: Verify `ServerState`, `ServerEvent`, and related types

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use std::future::Future;
use std::pin::Pin;

use toadstool::{
    ExecutionStatus, ResourceMonitor, RuntimeMetrics, RuntimeType, SystemResources, ToadStoolResult,
};
use toadstool_server::config::ServerConfig;
use toadstool_server::state::{
    ActiveExecution, ClientInfo, ServerEvent, ServerState, ServerStatistics,
};

// Simple mock for testing
struct MockResourceMonitor;

impl MockResourceMonitor {
    fn new() -> Self {
        Self
    }
}

impl ResourceMonitor for MockResourceMonitor {
    fn start_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    fn stop_monitoring(&self, _workload_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    fn get_metrics(
        &self,
        _workload_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async move { Ok(RuntimeMetrics::default()) })
    }

    fn get_system_resources(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemResources>> + Send + '_>> {
        Box::pin(async move {
            Ok(SystemResources {
                available_cpu_cores: 4.0,
                available_memory_bytes: 8_000_000_000,
                available_storage_bytes: 100_000_000_000,
                available_network_bandwidth: Some(1_000_000_000),
                available_gpu_units: 1,
                cpu_usage_percent: 25.0,
                memory_usage_percent: 50.0,
                total_cpu_cores: 8,
                total_memory_bytes: 16_000_000_000,
            })
        })
    }
}

// =============================================================================
// ServerEvent Tests
// =============================================================================

#[test]
fn test_server_event_to_json() {
    let execution_id = Uuid::new_v4();
    let timestamp = SystemTime::now();
    let event = ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type: RuntimeType::Native,
        timestamp,
    };
    let json = event.to_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "execution_started");
    assert!(parsed["data"]["execution_id"].is_string());
}

#[test]
fn test_server_event_execution_started() {
    let execution_id = Uuid::new_v4();
    let timestamp = SystemTime::now();

    let event = ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type: RuntimeType::Native,
        timestamp,
    };

    // Verify event can be created and cloned
    let cloned = event.clone();
    match cloned {
        ServerEvent::ExecutionStarted {
            execution_id: id,
            runtime_type,
            ..
        } => {
            assert_eq!(id, execution_id);
            assert_eq!(runtime_type, RuntimeType::Native);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_execution_completed() {
    let execution_id = Uuid::new_v4();
    let timestamp = SystemTime::now();

    let event = ServerEvent::ExecutionCompleted {
        execution_id,
        status: ExecutionStatus::Success,
        duration_ms: 5000,
        timestamp,
    };

    match event {
        ServerEvent::ExecutionCompleted {
            execution_id: id,
            status,
            duration_ms,
            ..
        } => {
            assert_eq!(id, execution_id);
            assert!(matches!(status, ExecutionStatus::Success));
            assert_eq!(duration_ms, 5000);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_execution_failed() {
    let execution_id = Uuid::new_v4();
    let timestamp = SystemTime::now();

    let event = ServerEvent::ExecutionCompleted {
        execution_id,
        status: ExecutionStatus::Failed {
            error: "Test error".into(),
        },
        duration_ms: 1000,
        timestamp,
    };

    match event {
        ServerEvent::ExecutionCompleted { status, .. } => match status {
            ExecutionStatus::Failed { error } => {
                assert_eq!(error, "Test error");
            }
            _ => panic!("Expected failed status"),
        },
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_runtime_engine_registered() {
    let timestamp = SystemTime::now();

    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: RuntimeType::Wasm,
        timestamp,
    };

    match event {
        ServerEvent::RuntimeEngineRegistered { runtime_type, .. } => {
            assert_eq!(runtime_type, RuntimeType::Wasm);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_resource_usage_update() {
    let timestamp = SystemTime::now();

    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 45.5,
        memory_usage_percent: 62.3,
        active_executions: 10,
        timestamp,
    };

    match event {
        ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent,
            memory_usage_percent,
            active_executions,
            ..
        } => {
            assert_eq!(cpu_usage_percent, 45.5);
            assert_eq!(memory_usage_percent, 62.3);
            assert_eq!(active_executions, 10);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_health_status_changed() {
    let timestamp = SystemTime::now();

    let event = ServerEvent::HealthStatusChanged {
        healthy: true,
        message: "System is healthy".to_string(),
        timestamp,
    };

    match event {
        ServerEvent::HealthStatusChanged {
            healthy, message, ..
        } => {
            assert!(healthy);
            assert_eq!(message, "System is healthy");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_error_occurred() {
    let execution_id = Uuid::new_v4();
    let timestamp = SystemTime::now();

    let event = ServerEvent::ErrorOccurred {
        error_type: "RuntimeError".to_string(),
        message: "Something went wrong".to_string(),
        execution_id: Some(execution_id),
        timestamp,
    };

    match event {
        ServerEvent::ErrorOccurred {
            error_type,
            message,
            execution_id: exec_id,
            ..
        } => {
            assert_eq!(error_type, "RuntimeError");
            assert_eq!(message, "Something went wrong");
            assert_eq!(exec_id, Some(execution_id));
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_server_event_clone() {
    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 50.0,
        memory_usage_percent: 60.0,
        active_executions: 5,
        timestamp: SystemTime::now(),
    };

    let cloned = event.clone();

    // Both should exist and be valid
    assert!(matches!(event, ServerEvent::ResourceUsageUpdate { .. }));
    assert!(matches!(cloned, ServerEvent::ResourceUsageUpdate { .. }));
}

// =============================================================================
// ActiveExecution Tests
// =============================================================================

#[test]
fn test_active_execution_creation() {
    let execution_id = Uuid::new_v4();
    let started_at = SystemTime::now();
    let timeout = Duration::from_secs(300);

    let execution = ActiveExecution {
        execution_id,
        runtime_type: RuntimeType::Native,
        started_at,
        timeout,
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: Some("ToadStool-Client/1.0".to_string()),
            api_key: None,
            authenticated_user: None,
        },
    };

    assert_eq!(execution.execution_id, execution_id);
    assert_eq!(execution.runtime_type, RuntimeType::Native);
    assert_eq!(execution.started_at, started_at);
    assert_eq!(execution.timeout, timeout);
    assert!(matches!(execution.status, ExecutionStatus::Running));
}

#[test]
fn test_active_execution_clone() {
    let execution = ActiveExecution {
        execution_id: Uuid::new_v4(),
        runtime_type: RuntimeType::Container,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(600),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: Some("Test-Agent".to_string()),
            api_key: Some("key-123".to_string()),
            authenticated_user: Some("user@example.com".to_string()),
        },
    };

    let cloned = execution.clone();

    assert_eq!(cloned.execution_id, execution.execution_id);
    assert_eq!(cloned.runtime_type, execution.runtime_type);
    assert_eq!(cloned.client_info.ip_address, Some("10.0.0.1".to_string()));
    assert_eq!(cloned.client_info.api_key, Some("key-123".to_string()));
}

// =============================================================================
// ClientInfo Tests
// =============================================================================

#[test]
fn test_client_info_with_all_fields() {
    let client_info = ClientInfo {
        ip_address: Some("203.0.113.42".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        api_key: Some("api-key-abc123".to_string()),
        authenticated_user: Some("admin@toadstool.dev".to_string()),
    };

    assert_eq!(client_info.ip_address, Some("203.0.113.42".to_string()));
    assert_eq!(client_info.user_agent, Some("Mozilla/5.0".to_string()));
    assert_eq!(client_info.api_key, Some("api-key-abc123".to_string()));
    assert_eq!(
        client_info.authenticated_user,
        Some("admin@toadstool.dev".to_string())
    );
}

#[test]
fn test_client_info_minimal() {
    let client_info = ClientInfo {
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: None,
        api_key: None,
        authenticated_user: None,
    };

    assert_eq!(client_info.ip_address, Some("127.0.0.1".to_string()));
    assert!(client_info.user_agent.is_none());
    assert!(client_info.api_key.is_none());
    assert!(client_info.authenticated_user.is_none());
}

#[test]
fn test_client_info_clone() {
    let client_info = ClientInfo {
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("Test".to_string()),
        api_key: Some("key".to_string()),
        authenticated_user: Some("user".to_string()),
    };

    let cloned = client_info.clone();

    assert_eq!(cloned.ip_address, client_info.ip_address);
    assert_eq!(cloned.user_agent, client_info.user_agent);
    assert_eq!(cloned.api_key, client_info.api_key);
    assert_eq!(cloned.authenticated_user, client_info.authenticated_user);
}

// =============================================================================
// ServerState Tests
// =============================================================================

#[test]
fn test_server_state_creation() {
    let config = ServerConfig::default();
    let resource_monitor = Arc::new(MockResourceMonitor::new());
    let (event_tx, _event_rx) = broadcast::channel(100);

    let state = ServerState {
        runtime_engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
        active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        event_broadcaster: event_tx,
        config,
        resource_monitor,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    };

    // Verify state was created successfully
    assert_eq!(state.config.max_concurrent_executions, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_event_broadcasting() {
    let config = ServerConfig::default();
    let resource_monitor = Arc::new(MockResourceMonitor::new());
    let (event_tx, mut event_rx) = broadcast::channel(100);

    let state = ServerState {
        runtime_engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
        active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        event_broadcaster: event_tx.clone(),
        config,
        resource_monitor,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    };

    // Send an event
    let test_event = ServerEvent::HealthStatusChanged {
        healthy: true,
        message: "Test message".to_string(),
        timestamp: SystemTime::now(),
    };

    let _ = state.event_broadcaster.send(test_event.clone());

    // Receive the event
    let received = event_rx.recv().await;
    assert!(received.is_ok());

    match received.unwrap() {
        ServerEvent::HealthStatusChanged {
            healthy, message, ..
        } => {
            assert!(healthy);
            assert_eq!(message, "Test message");
        }
        _ => panic!("Wrong event type received"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_active_executions() {
    let config = ServerConfig::default();
    let resource_monitor = Arc::new(MockResourceMonitor::new());
    let (event_tx, _event_rx) = broadcast::channel(100);

    let state = ServerState {
        runtime_engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
        active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        event_broadcaster: event_tx,
        config,
        resource_monitor,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    };

    // Add an active execution
    let execution_id = Uuid::new_v4();
    let execution = ActiveExecution {
        execution_id,
        runtime_type: RuntimeType::Native,
        started_at: SystemTime::now(),
        timeout: Duration::from_secs(300),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };

    {
        let mut active = state.active_executions.write().await;
        active.insert(execution_id, execution);
    }

    // Verify it was added
    let active = state.active_executions.read().await;
    assert_eq!(active.len(), 1);
    assert!(active.contains_key(&execution_id));
}

#[test]
fn test_server_state_clone() {
    let config = ServerConfig::default();
    let resource_monitor = Arc::new(MockResourceMonitor::new());
    let (event_tx, _event_rx) = broadcast::channel(100);

    let state = ServerState {
        runtime_engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
        active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        event_broadcaster: event_tx,
        config,
        resource_monitor,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    };

    // Clone the state
    let cloned = state.clone();

    // Both should be valid and share the same underlying data
    assert_eq!(
        cloned.config.max_concurrent_executions,
        state.config.max_concurrent_executions
    );
}
