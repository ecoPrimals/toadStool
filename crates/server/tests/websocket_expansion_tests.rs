//! Additional WebSocket tests to expand coverage from 52.46% → 80%+
//!
//! These tests complement websocket_comprehensive_tests.rs by covering:
//! - Additional ServerEvent types (RuntimeEngineRegistered, ErrorOccurred)
//! - Concurrent message handling scenarios
//! - Error edge cases
//! - Event broadcasting patterns

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool_server::{
    websocket::*, ClientInfo, ServerConfig, ServerEvent, ServerState, ServerStatistics,
};
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// Helper to create test server state
fn create_test_state() -> ServerState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config: ServerConfig::default(),
        resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    }
}

// ============================================================================
// Additional ServerEvent Formatting Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_server_event_runtime_engine_registered() {
    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: toadstool::RuntimeType::Wasm,
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Formatted event should be valid JSON");

    assert_eq!(parsed["type"], "runtime_engine_registered");
    assert!(parsed["data"]["runtime_type"].is_string());
    assert!(parsed["data"]["timestamp"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_server_event_error_occurred_with_execution_id() {
    let execution_id = Uuid::new_v4();
    let event = ServerEvent::ErrorOccurred {
        error_type: "EXECUTION_ERROR".to_string(),
        message: "Execution failed due to timeout".to_string(),
        execution_id: Some(execution_id),
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Formatted event should be valid JSON");

    assert_eq!(parsed["type"], "error_occurred");
    assert_eq!(parsed["data"]["error_type"], "EXECUTION_ERROR");
    assert_eq!(parsed["data"]["message"], "Execution failed due to timeout");
    assert_eq!(parsed["data"]["execution_id"], execution_id.to_string());
    assert!(parsed["data"]["timestamp"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_server_event_error_occurred_without_execution_id() {
    let event = ServerEvent::ErrorOccurred {
        error_type: "SYSTEM_ERROR".to_string(),
        message: "System resource exhausted".to_string(),
        execution_id: None,
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Formatted event should be valid JSON");

    assert_eq!(parsed["type"], "error_occurred");
    assert_eq!(parsed["data"]["error_type"], "SYSTEM_ERROR");
    assert_eq!(parsed["data"]["message"], "System resource exhausted");
    assert!(parsed["data"]["execution_id"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_all_server_event_types() {
    let execution_id = Uuid::new_v4();

    // Test all event types can be formatted without panicking
    let events = vec![
        ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: toadstool::RuntimeType::Native,
            timestamp: chrono::Utc::now(),
        },
        ServerEvent::ExecutionCompleted {
            execution_id,
            status: toadstool::ExecutionStatus::Success,
            duration_ms: 1000,
            timestamp: chrono::Utc::now(),
        },
        ServerEvent::RuntimeEngineRegistered {
            runtime_type: toadstool::RuntimeType::Wasm,
            timestamp: chrono::Utc::now(),
        },
        ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            active_executions: 3,
            timestamp: chrono::Utc::now(),
        },
        ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "OK".to_string(),
            timestamp: chrono::Utc::now(),
        },
        ServerEvent::ErrorOccurred {
            error_type: "TEST".to_string(),
            message: "Test error".to_string(),
            execution_id: Some(execution_id),
            timestamp: chrono::Utc::now(),
        },
    ];

    for event in events {
        let formatted = format_server_event(&event);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&formatted);
        assert!(parsed.is_ok(), "All events should format to valid JSON");
    }
}

// ============================================================================
// Message Handling Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_empty_string() {
    let state = create_test_state();
    let (tx, _rx) = mpsc::unbounded_channel();

    let result = handle_client_message("", &tx, &state).await;
    assert!(result.is_err(), "Empty string should fail JSON parsing");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_whitespace_only() {
    let state = create_test_state();
    let (tx, _rx) = mpsc::unbounded_channel();

    let result = handle_client_message("   ", &tx, &state).await;
    assert!(result.is_err(), "Whitespace should fail JSON parsing");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_null_json() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = handle_client_message("null", &tx, &state).await;
    assert!(result.is_ok(), "null is valid JSON");

    // Should get error response for missing type
    let response = rx.recv().await.expect("Should receive error response");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_array_json() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = handle_client_message("[1,2,3]", &tx, &state).await;
    assert!(result.is_ok(), "Array is valid JSON");

    // Should get error response for missing type
    let response = rx.recv().await.expect("Should receive error response");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_type_is_null() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({
        "type": null
    })
    .to_string();

    let result = handle_client_message(&message, &tx, &state).await;
    assert!(result.is_ok());

    // Should get error response
    let response = rx.recv().await.expect("Should receive error response");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_type_is_number() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({
        "type": 123
    })
    .to_string();

    let result = handle_client_message(&message, &tx, &state).await;
    assert!(result.is_ok());

    // Should get error response (type is not a string)
    let response = rx.recv().await.expect("Should receive error response");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_client_message_case_sensitive_type() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Type is case-sensitive, so "PING" should not match "ping"
    let message = json!({
        "type": "PING"
    })
    .to_string();

    let result = handle_client_message(&message, &tx, &state).await;
    assert!(result.is_ok());

    // Should get error response for unknown type
    let response = rx.recv().await.expect("Should receive error response");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "Unknown message type");
    }
}

// ============================================================================
// Concurrent Scenarios
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_message_handling() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Send multiple messages concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let tx_clone = tx.clone();
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let message = json!({
                "type": "ping",
                "id": i
            })
            .to_string();
            // Call and check result inline to avoid Send issues
            (handle_client_message(&message, &tx_clone, &state_clone).await).is_ok()
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        assert!(
            handle.await.unwrap(),
            "Message should be handled successfully"
        );
    }

    // Verify we received 10 responses
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 10, "Should receive 10 pong responses");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_status_requests_with_changing_state() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn task to add executions while status requests are being processed
    let state_clone = state.clone();
    let adder_task = tokio::spawn(async move {
        for i in 0..5 {
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
            let mut executions = state_clone.active_executions.write().await;
            let exec_id = Uuid::new_v4();
            executions.insert(
                exec_id,
                toadstool_server::ActiveExecution {
                    execution_id: exec_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: ClientInfo {
                        user_agent: Some(format!("test-{}", i)),
                        ip_address: Some("127.0.0.1".to_string()),
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    });

    // Send multiple status requests concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let tx_clone = tx.clone();
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let message = json!({"type": "get_status"}).to_string();
            (handle_client_message(&message, &tx_clone, &state_clone).await).is_ok()
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        assert!(handle.await.unwrap(), "Status request should succeed");
    }
    adder_task.await.unwrap();

    // Verify we got responses (counts may vary due to concurrent state changes)
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 10, "Should receive 10 status responses");
}

// ============================================================================
// State Verification Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_status_reflects_empty_state() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({"type": "get_status"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive status");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["data"]["active_executions"], 0);
        assert_eq!(parsed["data"]["runtime_engines"], 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_status_with_multiple_runtime_types() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add different runtime engine types
    {
        let mut engines = state.runtime_engines.write().await;
        use toadstool_testing::mocks::MockRuntimeEngine;
        engines.insert(
            toadstool::RuntimeType::Native,
            Box::new(MockRuntimeEngine::new()),
        );
        engines.insert(
            toadstool::RuntimeType::Wasm,
            Box::new(MockRuntimeEngine::new()),
        );
        engines.insert(
            toadstool::RuntimeType::Container,
            Box::new(MockRuntimeEngine::new()),
        );
    }

    let message = json!({"type": "get_status"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive status");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["data"]["runtime_engines"], 3);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_status_with_multiple_execution_types() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add executions with different runtime types
    {
        let mut executions = state.active_executions.write().await;
        let runtime_types = vec![
            toadstool::RuntimeType::Native,
            toadstool::RuntimeType::Wasm,
            toadstool::RuntimeType::Container,
            toadstool::RuntimeType::Python,
        ];

        for runtime_type in runtime_types {
            let exec_id = Uuid::new_v4();
            executions.insert(
                exec_id,
                toadstool_server::ActiveExecution {
                    execution_id: exec_id,
                    runtime_type,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(300),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: ClientInfo {
                        user_agent: Some("test".to_string()),
                        ip_address: Some("127.0.0.1".to_string()),
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    let message = json!({"type": "get_status"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive status");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["data"]["active_executions"], 4);
    }
}

// ============================================================================
// Message Type Variations
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ping_response_includes_timestamp() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({"type": "ping"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive pong");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "pong");

        // Verify timestamp is a valid ISO 8601 string
        let timestamp_str = parsed["timestamp"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(timestamp_str).is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_subscribe_response_format() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({"type": "subscribe"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive subscribed");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "subscribed");
        assert!(parsed["message"].is_string());
        assert!(parsed["timestamp"].is_string());
        assert!(!parsed["message"].as_str().unwrap().is_empty());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_error_response_format() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let message = json!({"type": "invalid_type"}).to_string();
    handle_client_message(&message, &tx, &state).await.unwrap();

    let response = rx.recv().await.expect("Should receive error");
    if let axum::extract::ws::Message::Text(text) = response {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "Unknown message type");
        assert!(parsed["timestamp"].is_string());
    }
}

// ============================================================================
// Event Formatting Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_event_with_special_characters() {
    let event = ServerEvent::ErrorOccurred {
        error_type: "ERROR".to_string(),
        message: "Error with \"quotes\" and 'apostrophes' and \n newlines".to_string(),
        execution_id: None,
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&formatted);
    assert!(parsed.is_ok(), "Should handle special characters in JSON");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_event_with_unicode() {
    let event = ServerEvent::HealthStatusChanged {
        healthy: true,
        message: "Status: ✅ 🚀 測試".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Should handle Unicode characters");

    assert!(parsed["data"]["message"].as_str().unwrap().contains("✅"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_event_with_empty_strings() {
    let event = ServerEvent::ErrorOccurred {
        error_type: "".to_string(),
        message: "".to_string(),
        execution_id: None,
        timestamp: chrono::Utc::now(),
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value =
        serde_json::from_str(&formatted).expect("Should handle empty strings");

    assert_eq!(parsed["data"]["error_type"], "");
    assert_eq!(parsed["data"]["message"], "");
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_many_sequential_messages() {
    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Send 100 messages
    for i in 0..100 {
        let message = json!({
            "type": "ping",
            "id": i
        })
        .to_string();

        let result = handle_client_message(&message, &tx, &state).await;
        assert!(result.is_ok());
    }

    // Verify we got 100 responses
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_many_events_sequentially() {
    let execution_id = Uuid::new_v4();

    // Format 100 events
    for i in 0..100 {
        let event = ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type: toadstool::RuntimeType::Native,
            timestamp: chrono::Utc::now(),
        };

        let formatted = format_server_event(&event);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&formatted);
        assert!(parsed.is_ok(), "Event {} should format correctly", i);
    }
}
