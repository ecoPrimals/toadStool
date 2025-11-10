//! Comprehensive tests for WebSocket module
//!
//! This test suite achieves 60%+ coverage of server/websocket.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};

use axum::extract::ws::Message;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

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
mod format_server_event_tests {
    use super::*;

    #[test]
    fn test_format_execution_started_event() {
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
    }

    #[test]
    fn test_format_execution_completed_event() {
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
    }

    #[test]
    fn test_format_runtime_engine_registered_event() {
        let timestamp = Utc::now();

        let event = ServerEvent::RuntimeEngineRegistered {
            runtime_type: RuntimeType::Python,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "runtime_engine_registered");
        assert_eq!(parsed["data"]["runtime_type"], "Python");
    }

    #[test]
    fn test_format_resource_usage_update_event() {
        let timestamp = Utc::now();

        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 45.5,
            memory_usage_percent: 67.8,
            active_executions: 3,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "resource_usage_update");
        assert_eq!(parsed["data"]["cpu_usage_percent"], 45.5);
        assert_eq!(parsed["data"]["memory_usage_percent"], 67.8);
        assert_eq!(parsed["data"]["active_executions"], 3);
    }

    #[test]
    fn test_format_health_status_changed_event() {
        let timestamp = Utc::now();

        let event = ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "System is healthy".to_string(),
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "health_status_changed");
        assert_eq!(parsed["data"]["healthy"], true);
        assert_eq!(parsed["data"]["message"], "System is healthy");
    }

    #[test]
    fn test_format_health_status_changed_unhealthy() {
        let timestamp = Utc::now();

        let event = ServerEvent::HealthStatusChanged {
            healthy: false,
            message: "System is unhealthy".to_string(),
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "health_status_changed");
        assert_eq!(parsed["data"]["healthy"], false);
        assert_eq!(parsed["data"]["message"], "System is unhealthy");
    }

    #[test]
    fn test_format_error_occurred_event() {
        let execution_id = Some(Uuid::new_v4());
        let timestamp = Utc::now();

        let event = ServerEvent::ErrorOccurred {
            error_type: "RuntimeError".to_string(),
            message: "Something went wrong".to_string(),
            execution_id,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "error_occurred");
        assert_eq!(parsed["data"]["error_type"], "RuntimeError");
        assert_eq!(parsed["data"]["message"], "Something went wrong");
        assert!(parsed["data"]["execution_id"].is_string());
    }

    #[test]
    fn test_format_error_occurred_without_execution_id() {
        let timestamp = Utc::now();

        let event = ServerEvent::ErrorOccurred {
            error_type: "SystemError".to_string(),
            message: "General error".to_string(),
            execution_id: None,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["type"], "error_occurred");
        assert_eq!(parsed["data"]["error_type"], "SystemError");
        assert!(parsed["data"]["execution_id"].is_null());
    }

    #[test]
    fn test_formatted_events_are_valid_json() {
        let timestamp = Utc::now();
        let events = vec![
            ServerEvent::ExecutionStarted {
                execution_id: Uuid::new_v4(),
                runtime_type: RuntimeType::Native,
                timestamp,
            },
            ServerEvent::ResourceUsageUpdate {
                cpu_usage_percent: 50.0,
                memory_usage_percent: 60.0,
                active_executions: 5,
                timestamp,
            },
            ServerEvent::HealthStatusChanged {
                healthy: true,
                message: "OK".to_string(),
                timestamp,
            },
        ];

        for event in events {
            let formatted = format_server_event(&event);
            assert!(
                serde_json::from_str::<serde_json::Value>(&formatted).is_ok(),
                "Formatted event should be valid JSON"
            );
        }
    }

    #[test]
    fn test_formatted_events_have_timestamp() {
        let timestamp = Utc::now();
        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            active_executions: 5,
            timestamp,
        };

        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert!(parsed["data"]["timestamp"].is_string());
    }
}

#[cfg(test)]
mod handle_client_message_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_ping_message() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "ping"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Ping message should be handled successfully"
        );

        // Check response
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
    async fn test_handle_get_status_message() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "get_status"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Get status message should be handled successfully"
        );

        // Check response
        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "status");
                assert!(parsed["data"]["active_executions"].is_number());
                assert!(parsed["data"]["runtime_engines"].is_number());
                assert!(parsed["data"]["timestamp"].is_string());
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_status_with_active_executions() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Add some active executions
        {
            let mut executions = state.active_executions.write().await;
            let exec_id = Uuid::new_v4();
            executions.insert(
                exec_id,
                toadstool_server::state::ActiveExecution {
                    execution_id: exec_id,
                    runtime_type: RuntimeType::Native,
                    started_at: Utc::now(),
                    timeout: Duration::from_secs(30),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let message = json!({"type": "get_status"}).to_string();
        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok());

        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["data"]["active_executions"], 1);
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_subscribe_message() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "subscribe"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Subscribe message should be handled successfully"
        );

        // Check response
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

    #[tokio::test]
    async fn test_handle_unknown_message_type() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"type": "unknown_command"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Unknown message should return error response"
        );

        // Check response
        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "error");
                assert_eq!(parsed["message"], "Unknown message type");
                assert!(parsed["timestamp"].is_string());
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_message_without_type() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({"data": "some data"}).to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(
            result.is_ok(),
            "Message without type should return error response"
        );

        // Check response
        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "error");
                assert_eq!(parsed["message"], "Unknown message type");
            }
            _ => panic!("Expected text message"),
        }
    }

    #[tokio::test]
    async fn test_handle_invalid_json() {
        let state = create_test_state();
        let (tx, _rx) = mpsc::unbounded_channel();

        let message = "not valid json";

        let result = handle_client_message(message, &tx, &state).await;
        assert!(result.is_err(), "Invalid JSON should return error");
    }

    #[tokio::test]
    async fn test_handle_multiple_messages_sequentially() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Send ping
        let ping = json!({"type": "ping"}).to_string();
        handle_client_message(&ping, &tx, &state).await.unwrap();

        // Send get_status
        let status = json!({"type": "get_status"}).to_string();
        handle_client_message(&status, &tx, &state).await.unwrap();

        // Send subscribe
        let subscribe = json!({"type": "subscribe"}).to_string();
        handle_client_message(&subscribe, &tx, &state)
            .await
            .unwrap();

        // Verify all responses
        let mut response_types = Vec::new();
        for _ in 0..3 {
            if let Some(Message::Text(text)) = rx.recv().await {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                response_types.push(parsed["type"].as_str().unwrap().to_string());
            }
        }

        assert!(response_types.contains(&"pong".to_string()));
        assert!(response_types.contains(&"status".to_string()));
        assert!(response_types.contains(&"subscribed".to_string()));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_message_handling_preserves_state() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Add execution to state
        {
            let mut executions = state.active_executions.write().await;
            let exec_id = Uuid::new_v4();
            executions.insert(
                exec_id,
                toadstool_server::state::ActiveExecution {
                    execution_id: exec_id,
                    runtime_type: RuntimeType::Native,
                    started_at: Utc::now(),
                    timeout: Duration::from_secs(30),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        // Request status multiple times
        for _ in 0..3 {
            let message = json!({"type": "get_status"}).to_string();
            handle_client_message(&message, &tx, &state).await.unwrap();
        }

        // Verify state is consistent
        for _ in 0..3 {
            if let Some(Message::Text(text)) = rx.recv().await {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["data"]["active_executions"], 1);
            }
        }
    }

    #[tokio::test]
    async fn test_format_and_parse_round_trip() {
        let execution_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: RuntimeType::Python,
            timestamp,
        };

        // Format the event
        let formatted = format_server_event(&event);

        // Parse it back
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        // Verify round-trip
        assert_eq!(parsed["type"], "execution_started");
        assert_eq!(parsed["data"]["runtime_type"], "Python");
    }

    #[tokio::test]
    async fn test_all_message_types_produce_responses() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message_types = vec!["ping", "get_status", "subscribe", "unknown"];

        for msg_type in message_types {
            let message = json!({"type": msg_type}).to_string();
            handle_client_message(&message, &tx, &state).await.unwrap();
        }

        // Verify we got 4 responses
        let mut count = 0;
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            count += 1;
        }

        assert_eq!(count, 4, "Should receive response for each message type");
    }

    #[tokio::test]
    async fn test_concurrent_message_handling() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Send multiple messages concurrently (using join instead of spawn)
        let futures: Vec<_> = (0..10)
            .map(|_| {
                let state = state.clone();
                let tx = tx.clone();
                async move {
                    let message = json!({"type": "ping"}).to_string();
                    handle_client_message(&message, &tx, &state)
                        .await
                        .map_err(|_| ())
                }
            })
            .collect();

        // Wait for all to complete
        let results = futures_util::future::join_all(futures).await;
        for result in results {
            assert!(result.is_ok(), "All messages should be handled");
        }

        // Count responses
        let mut count = 0;
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            count += 1;
        }

        assert_eq!(count, 10, "Should handle concurrent messages correctly");
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_empty_json_object() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = "{}".to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok(), "Empty JSON should be handled");

        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "error");
            }
            _ => panic!("Expected error response"),
        }
    }

    #[tokio::test]
    async fn test_format_event_with_special_characters() {
        let timestamp = Utc::now();
        let event = ServerEvent::ErrorOccurred {
            error_type: "Error with \"quotes\"".to_string(),
            message: "Message with\nnewlines".to_string(),
            execution_id: None,
            timestamp,
        };

        let formatted = format_server_event(&event);

        // Should still be valid JSON
        assert!(
            serde_json::from_str::<serde_json::Value>(&formatted).is_ok(),
            "Should handle special characters in JSON"
        );
    }

    #[tokio::test]
    async fn test_handle_message_with_extra_fields() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let message = json!({
            "type": "ping",
            "extra_field": "ignored",
            "another": 123
        })
        .to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok(), "Extra fields should be ignored");

        let response = rx.recv().await.unwrap();
        match response {
            Message::Text(text) => {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(parsed["type"], "pong");
            }
            _ => panic!("Expected pong response"),
        }
    }
}
