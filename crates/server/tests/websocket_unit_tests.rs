//! Unit tests for WebSocket module that properly exercise the library code
//!
//! These tests are designed to achieve 50%+ coverage of server/websocket.rs

use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use axum::extract::ws::Message;
use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::config::ServerConfig;
use toadstool_server::state::{ServerEvent, ServerState, ServerStatistics};
use toadstool_server::websocket::{format_server_event, handle_client_message};
use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;

/// Helper to create a test ServerState
fn create_test_state() -> ServerState {
    let config = ServerConfig::default();
    let (event_broadcaster, _) = broadcast::channel(100);

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config,
        resource_monitor: Arc::new(MockResourceMonitor::new_successful()),
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
    }
}

#[cfg(test)]
mod format_server_event_unit_tests {
    use super::*;

    #[test]
    fn test_format_execution_started_event_native() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: RuntimeType::Native,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "execution_started");
        assert_eq!(parsed["data"]["execution_id"], execution_id.to_string());
        assert_eq!(parsed["data"]["runtime_type"], "Native");
        assert!(parsed["data"]["timestamp"].is_string());
    }

    #[test]
    fn test_format_execution_started_event_python() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: RuntimeType::Python,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "execution_started");
        assert_eq!(parsed["data"]["runtime_type"], "Python");
    }

    #[test]
    fn test_format_execution_started_event_wasm() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: RuntimeType::Wasm,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "execution_started");
        assert_eq!(parsed["data"]["runtime_type"], "Wasm");
    }

    #[test]
    fn test_format_execution_completed_success() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionCompleted {
            execution_id,
            status: ExecutionStatus::Success,
            duration_ms: 1234,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "execution_completed");
        assert_eq!(parsed["data"]["execution_id"], execution_id.to_string());
        assert_eq!(parsed["data"]["duration_ms"], 1234);
        assert!(parsed["data"]["status"].is_string());
    }

    #[test]
    fn test_format_execution_completed_failure() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionCompleted {
            execution_id,
            status: ExecutionStatus::Failed {
                error: "Execution failed with error".to_string(),
            },
            duration_ms: 567,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "execution_completed");
        assert_eq!(parsed["data"]["duration_ms"], 567);
    }

    #[test]
    fn test_format_runtime_engine_registered_python() {
        let timestamp = Utc::now();

        let event = ServerEvent::RuntimeEngineRegistered {
            runtime_type: RuntimeType::Python,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "runtime_engine_registered");
        assert_eq!(parsed["data"]["runtime_type"], "Python");
        assert!(parsed["data"]["timestamp"].is_string());
    }

    #[test]
    fn test_format_resource_usage_update_high_load() {
        let timestamp = Utc::now();

        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 89.5,
            memory_usage_percent: 95.2,
            active_executions: 10,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "resource_usage_update");
        assert_eq!(parsed["data"]["cpu_usage_percent"], 89.5);
        assert_eq!(parsed["data"]["memory_usage_percent"], 95.2);
        assert_eq!(parsed["data"]["active_executions"], 10);
    }

    #[test]
    fn test_format_resource_usage_update_low_load() {
        let timestamp = Utc::now();

        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 5.1,
            memory_usage_percent: 10.3,
            active_executions: 0,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "resource_usage_update");
        assert_eq!(parsed["data"]["cpu_usage_percent"], 5.1);
        assert_eq!(parsed["data"]["memory_usage_percent"], 10.3);
        assert_eq!(parsed["data"]["active_executions"], 0);
    }

    #[test]
    fn test_format_health_status_changed_healthy() {
        let timestamp = Utc::now();

        let event = ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "All systems operational".to_string(),
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "health_status_changed");
        assert_eq!(parsed["data"]["healthy"], true);
        assert_eq!(parsed["data"]["message"], "All systems operational");
    }

    #[test]
    fn test_format_health_status_changed_unhealthy() {
        let timestamp = Utc::now();

        let event = ServerEvent::HealthStatusChanged {
            healthy: false,
            message: "Resource exhaustion detected".to_string(),
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "health_status_changed");
        assert_eq!(parsed["data"]["healthy"], false);
        assert_eq!(parsed["data"]["message"], "Resource exhaustion detected");
    }

    #[test]
    fn test_format_error_occurred_with_execution_id() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ErrorOccurred {
            error_type: "RuntimeError".to_string(),
            message: "Python execution timeout".to_string(),
            execution_id: Some(execution_id),
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "error_occurred");
        assert_eq!(parsed["data"]["error_type"], "RuntimeError");
        assert_eq!(parsed["data"]["message"], "Python execution timeout");
        assert_eq!(
            parsed["data"]["execution_id"],
            serde_json::Value::String(execution_id.to_string())
        );
    }

    #[test]
    fn test_format_error_occurred_without_execution_id() {
        let timestamp = Utc::now();

        let event = ServerEvent::ErrorOccurred {
            error_type: "SystemError".to_string(),
            message: "Disk space critically low".to_string(),
            execution_id: None,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "error_occurred");
        assert_eq!(parsed["data"]["error_type"], "SystemError");
        assert_eq!(parsed["data"]["message"], "Disk space critically low");
        assert_eq!(parsed["data"]["execution_id"], serde_json::Value::Null);
    }
}

#[cfg(test)]
mod handle_client_message_unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_ping_message_unit() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "ping"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Ping message should be handled successfully"
        );

        // Verify response
        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "pong");
                assert!(parsed["timestamp"].is_string());
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_status_empty_state() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "get_status"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok());

        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "status");
                assert_eq!(parsed["data"]["active_executions"], 0);
                assert_eq!(parsed["data"]["runtime_engines"], 0);
                assert!(parsed["data"]["timestamp"].is_string());
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_subscribe_message_unit() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "subscribe"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok());

        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "subscribed");
                assert_eq!(parsed["message"], "Subscribed to server events");
                assert!(parsed["timestamp"].is_string());
            }
            _ => panic!("Expected text message"),
        }
    }
}
