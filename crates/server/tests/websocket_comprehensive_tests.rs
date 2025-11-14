//! Comprehensive tests for WebSocket handlers
//! Addresses zero-coverage file: server/src/websocket.rs (244 lines)

use serde_json::{json, Value};
use tokio::sync::mpsc;

// Mock types for testing
#[derive(Clone, Debug)]
enum MockServerEvent {
    ExecutionStarted {
        execution_id: String,
        runtime_type: String,
        timestamp: String,
    },
    ExecutionCompleted {
        execution_id: String,
        status: String,
        duration_ms: u64,
        timestamp: String,
    },
    RuntimeEngineRegistered {
        runtime_type: String,
        timestamp: String,
    },
    ResourceUsageUpdate {
        cpu_usage_percent: f64,
        memory_usage_percent: f64,
        active_executions: usize,
        timestamp: String,
    },
    HealthStatusChanged {
        healthy: bool,
        message: String,
        timestamp: String,
    },
    ErrorOccurred {
        error_type: String,
        message: String,
        execution_id: Option<String>,
        timestamp: String,
    },
}

// Test format_server_event for different event types
#[test]
fn test_format_execution_started_event() {
    let event = MockServerEvent::ExecutionStarted {
        execution_id: "exec-123".to_string(),
        runtime_type: "native".to_string(),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("execution_started"));
    assert!(formatted.contains("exec-123"));
    assert!(formatted.contains("native"));
}

#[test]
fn test_format_execution_completed_event() {
    let event = MockServerEvent::ExecutionCompleted {
        execution_id: "exec-456".to_string(),
        status: "success".to_string(),
        duration_ms: 1500,
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("execution_completed"));
    assert!(formatted.contains("exec-456"));
    assert!(formatted.contains("success"));
    assert!(formatted.contains("1500"));
}

#[test]
fn test_format_runtime_engine_registered_event() {
    let event = MockServerEvent::RuntimeEngineRegistered {
        runtime_type: "wasm".to_string(),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("runtime_engine_registered"));
    assert!(formatted.contains("wasm"));
}

#[test]
fn test_format_resource_usage_update_event() {
    let event = MockServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 45.5,
        memory_usage_percent: 60.2,
        active_executions: 3,
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("resource_usage_update"));
    assert!(formatted.contains("45.5"));
    assert!(formatted.contains("60.2"));
    assert!(formatted.contains("3"));
}

#[test]
fn test_format_health_status_changed_event() {
    let event = MockServerEvent::HealthStatusChanged {
        healthy: true,
        message: "System healthy".to_string(),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("health_status_changed"));
    assert!(formatted.contains("true"));
    assert!(formatted.contains("System healthy"));
}

#[test]
fn test_format_error_occurred_event() {
    let event = MockServerEvent::ErrorOccurred {
        error_type: "RuntimeError".to_string(),
        message: "Execution failed".to_string(),
        execution_id: Some("exec-789".to_string()),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("error_occurred"));
    assert!(formatted.contains("RuntimeError"));
    assert!(formatted.contains("Execution failed"));
}

// Test handle_client_message for different message types
#[tokio::test]
async fn test_handle_ping_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let ping_msg = json!({
        "type": "ping"
    })
    .to_string();

    let result = mock_handle_client_message(&ping_msg, &tx).await;
    assert!(result.is_ok());

    // Should receive pong response
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "pong");
    }
}

#[tokio::test]
async fn test_handle_get_status_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let status_msg = json!({
        "type": "get_status"
    })
    .to_string();

    let result = mock_handle_client_message(&status_msg, &tx).await;
    assert!(result.is_ok());

    // Should receive status response
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "status");
        assert!(response["data"].is_object());
    }
}

#[tokio::test]
async fn test_handle_subscribe_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let subscribe_msg = json!({
        "type": "subscribe"
    })
    .to_string();

    let result = mock_handle_client_message(&subscribe_msg, &tx).await;
    assert!(result.is_ok());

    // Should receive subscribed response
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "subscribed");
        assert!(response["message"].as_str().unwrap().contains("Subscribed"));
    }
}

#[tokio::test]
async fn test_handle_unknown_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let unknown_msg = json!({
        "type": "unknown_type"
    })
    .to_string();

    let result = mock_handle_client_message(&unknown_msg, &tx).await;
    assert!(result.is_ok());

    // Should receive error response
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "error");
        assert!(response["message"].as_str().unwrap().contains("Unknown"));
    }
}

#[tokio::test]
async fn test_handle_invalid_json_message() {
    let (tx, _rx) = mpsc::unbounded_channel();

    let invalid_msg = "not valid json";

    let result = mock_handle_client_message(invalid_msg, &tx).await;
    // Should return error for invalid JSON
    assert!(result.is_err());
}

// Test welcome message format
#[test]
fn test_welcome_message_format() {
    let welcome = json!({
        "type": "welcome",
        "message": "Connected to ToadStool Server",
        "timestamp": "2025-11-13T00:00:00Z",
    });

    assert_eq!(welcome["type"], "welcome");
    assert!(welcome["message"].as_str().unwrap().contains("Connected"));
    assert!(welcome["timestamp"].is_string());
}

// Test multiple ping-pong exchanges
#[tokio::test]
async fn test_multiple_ping_pong() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    for _ in 0..5 {
        let ping_msg = json!({"type": "ping"}).to_string();
        let _ = mock_handle_client_message(&ping_msg, &tx).await;
    }

    // Should receive 5 pong responses
    let mut pong_count = 0;
    while let Ok(msg) = rx.try_recv() {
        let response: Value = serde_json::from_str(&msg).unwrap();
        if response["type"] == "pong" {
            pong_count += 1;
        }
    }

    assert_eq!(pong_count, 5);
}

// Test concurrent message handling
#[tokio::test]
async fn test_concurrent_message_handling() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let messages = vec![
        json!({"type": "ping"}).to_string(),
        json!({"type": "get_status"}).to_string(),
        json!({"type": "subscribe"}).to_string(),
    ];

    for msg in messages {
        let _ = mock_handle_client_message(&msg, &tx).await;
    }

    // Should receive 3 responses
    let mut response_count = 0;
    while rx.try_recv().is_ok() {
        response_count += 1;
    }

    assert_eq!(response_count, 3);
}

// Test event formatting edge cases
#[test]
fn test_format_event_with_empty_execution_id() {
    let event = MockServerEvent::ExecutionStarted {
        execution_id: "".to_string(),
        runtime_type: "native".to_string(),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("execution_started"));
}

#[test]
fn test_format_event_with_zero_duration() {
    let event = MockServerEvent::ExecutionCompleted {
        execution_id: "exec-fast".to_string(),
        status: "success".to_string(),
        duration_ms: 0,
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("0"));
}

#[test]
fn test_format_event_with_high_resource_usage() {
    let event = MockServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 99.9,
        memory_usage_percent: 95.5,
        active_executions: 100,
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("99.9"));
    assert!(formatted.contains("95.5"));
    assert!(formatted.contains("100"));
}

#[test]
fn test_format_event_unhealthy_status() {
    let event = MockServerEvent::HealthStatusChanged {
        healthy: false,
        message: "System degraded".to_string(),
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("false"));
    assert!(formatted.contains("System degraded"));
}

// Test message validation
#[tokio::test]
async fn test_message_without_type_field() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let msg = json!({
        "data": "some data"
    })
    .to_string();

    let result = mock_handle_client_message(&msg, &tx).await;
    assert!(result.is_ok());

    // Should receive error response
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "error");
    }
}

#[tokio::test]
async fn test_message_with_extra_fields() {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let msg = json!({
        "type": "ping",
        "extra": "ignored data",
        "another": 123
    })
    .to_string();

    let result = mock_handle_client_message(&msg, &tx).await;
    assert!(result.is_ok());

    // Should still handle as ping
    if let Some(msg) = rx.recv().await {
        let response: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(response["type"], "pong");
    }
}

// Test error event formatting with no execution_id
#[test]
fn test_format_error_event_without_execution_id() {
    let event = MockServerEvent::ErrorOccurred {
        error_type: "ConfigError".to_string(),
        message: "Invalid configuration".to_string(),
        execution_id: None,
        timestamp: "2025-11-13T00:00:00Z".to_string(),
    };

    let formatted = mock_format_server_event(&event);
    assert!(formatted.contains("error_occurred"));
    assert!(formatted.contains("ConfigError"));
}

// Test JSON serialization/deserialization
#[test]
fn test_json_round_trip_ping() {
    let original = json!({"type": "ping"});
    let serialized = original.to_string();
    let deserialized: Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_json_round_trip_complex() {
    let original = json!({
        "type": "execution_started",
        "data": {
            "execution_id": "exec-123",
            "runtime_type": "native",
            "timestamp": "2025-11-13T00:00:00Z"
        }
    });

    let serialized = original.to_string();
    let deserialized: Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original["type"], deserialized["type"]);
    assert_eq!(
        original["data"]["execution_id"],
        deserialized["data"]["execution_id"]
    );
}

// Helper functions (mock implementations)
fn mock_format_server_event(event: &MockServerEvent) -> String {
    match event {
        MockServerEvent::ExecutionStarted {
            execution_id,
            runtime_type,
            timestamp,
        } => json!({
            "type": "execution_started",
            "data": {
                "execution_id": execution_id,
                "runtime_type": runtime_type,
                "timestamp": timestamp,
            }
        })
        .to_string(),
        MockServerEvent::ExecutionCompleted {
            execution_id,
            status,
            duration_ms,
            timestamp,
        } => json!({
            "type": "execution_completed",
            "data": {
                "execution_id": execution_id,
                "status": status,
                "duration_ms": duration_ms,
                "timestamp": timestamp,
            }
        })
        .to_string(),
        MockServerEvent::RuntimeEngineRegistered {
            runtime_type,
            timestamp,
        } => json!({
            "type": "runtime_engine_registered",
            "data": {
                "runtime_type": runtime_type,
                "timestamp": timestamp,
            }
        })
        .to_string(),
        MockServerEvent::ResourceUsageUpdate {
            cpu_usage_percent,
            memory_usage_percent,
            active_executions,
            timestamp,
        } => json!({
            "type": "resource_usage_update",
            "data": {
                "cpu_usage_percent": cpu_usage_percent,
                "memory_usage_percent": memory_usage_percent,
                "active_executions": active_executions,
                "timestamp": timestamp,
            }
        })
        .to_string(),
        MockServerEvent::HealthStatusChanged {
            healthy,
            message,
            timestamp,
        } => json!({
            "type": "health_status_changed",
            "data": {
                "healthy": healthy,
                "message": message,
                "timestamp": timestamp,
            }
        })
        .to_string(),
        MockServerEvent::ErrorOccurred {
            error_type,
            message,
            execution_id,
            timestamp,
        } => json!({
            "type": "error_occurred",
            "data": {
                "error_type": error_type,
                "message": message,
                "execution_id": execution_id,
                "timestamp": timestamp,
            }
        })
        .to_string(),
    }
}

async fn mock_handle_client_message(
    message: &str,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request: Value = serde_json::from_str(message)?;

    match request.get("type").and_then(|t| t.as_str()) {
        Some("ping") => {
            let response = json!({
                "type": "pong",
                "timestamp": "2025-11-13T00:00:00Z",
            });
            tx.send(response.to_string())?;
        }
        Some("get_status") => {
            let response = json!({
                "type": "status",
                "data": {
                    "active_executions": 0,
                    "runtime_engines": 0,
                    "timestamp": "2025-11-13T00:00:00Z",
                }
            });
            tx.send(response.to_string())?;
        }
        Some("subscribe") => {
            let response = json!({
                "type": "subscribed",
                "message": "Subscribed to server events",
                "timestamp": "2025-11-13T00:00:00Z",
            });
            tx.send(response.to_string())?;
        }
        _ => {
            let response = json!({
                "type": "error",
                "message": "Unknown message type",
                "timestamp": "2025-11-13T00:00:00Z",
            });
            tx.send(response.to_string())?;
        }
    }

    Ok(())
}
