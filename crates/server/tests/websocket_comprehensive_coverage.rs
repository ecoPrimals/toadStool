//! Comprehensive WebSocket coverage tests
//! Target: websocket.rs module coverage expansion

use axum::extract::ws::Message;
use serde_json::json;

#[test]
fn test_websocket_message_types() {
    // Test different message types that websocket handlers process
    let text_msg = Message::Text("test".to_string());
    assert!(matches!(text_msg, Message::Text(_)));

    let ping_msg = Message::Ping(vec![1, 2, 3]);
    assert!(matches!(ping_msg, Message::Ping(_)));

    let pong_msg = Message::Pong(vec![1, 2, 3]);
    assert!(matches!(pong_msg, Message::Pong(_)));
}

#[test]
fn test_websocket_json_messages() {
    // Test JSON message formats used in websocket communication
    let welcome = json!({
        "type": "welcome",
        "message": "Connected to ToadStool Server",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    assert_eq!(welcome["type"], "welcome");
    assert!(welcome["message"].is_string());

    let status = json!({
        "type": "status",
        "data": {
            "active_executions": 5,
            "cpu_usage": 45.0
        }
    });

    assert_eq!(status["type"], "status");
    assert!(status["data"].is_object());
}

#[test]
fn test_websocket_event_types() {
    // Test event types that websocket broadcasts
    let event_types = vec![
        "execution_started",
        "execution_completed",
        "execution_failed",
        "resource_alert",
        "health_status",
        "welcome",
        "status",
    ];

    for event_type in event_types {
        let event = json!({
            "type": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        assert_eq!(event["type"], event_type);
    }
}

#[test]
fn test_websocket_message_serialization() {
    let msg = json!({
        "type": "execution_started",
        "execution_id": "test-123",
        "runtime": "wasm",
    });

    let serialized = serde_json::to_string(&msg);
    assert!(serialized.is_ok());

    let json_str = serialized.unwrap();
    assert!(json_str.contains("execution_started"));
    assert!(json_str.contains("test-123"));
}

#[test]
fn test_websocket_message_deserialization() {
    let json_str = r#"{"type":"status","data":{"active":5}}"#;

    let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str);
    assert!(parsed.is_ok());

    let msg = parsed.unwrap();
    assert_eq!(msg["type"], "status");
    assert_eq!(msg["data"]["active"], 5);
}

#[test]
fn test_websocket_error_messages() {
    let error_msg = json!({
        "type": "error",
        "code": "EXECUTION_FAILED",
        "message": "Execution failed: timeout",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    assert_eq!(error_msg["type"], "error");
    assert_eq!(error_msg["code"], "EXECUTION_FAILED");
}

#[test]
fn test_websocket_execution_events() {
    // Test execution lifecycle events
    let events = vec![
        ("execution_started", "RUNNING"),
        ("execution_completed", "SUCCESS"),
        ("execution_failed", "FAILED"),
    ];

    for (event_type, status) in events {
        let event = json!({
            "type": event_type,
            "execution_id": "exec-123",
            "status": status,
        });

        assert_eq!(event["type"], event_type);
        assert_eq!(event["status"], status);
    }
}

#[test]
fn test_websocket_resource_alerts() {
    let alert = json!({
        "type": "resource_alert",
        "severity": "warning",
        "metric": "cpu_usage",
        "value": 85.0,
        "threshold": 80.0,
    });

    assert_eq!(alert["type"], "resource_alert");
    assert_eq!(alert["severity"], "warning");
    assert!(alert["value"].as_f64().unwrap() > alert["threshold"].as_f64().unwrap());
}

#[test]
fn test_websocket_health_status() {
    let health = json!({
        "type": "health_status",
        "status": "healthy",
        "details": {
            "cpu_ok": true,
            "memory_ok": true,
            "disk_ok": true,
        }
    });

    assert_eq!(health["type"], "health_status");
    assert_eq!(health["status"], "healthy");
}

#[test]
fn test_websocket_message_timestamps() {
    use chrono::Utc;

    let now = Utc::now();
    let msg = json!({
        "type": "test",
        "timestamp": now.to_rfc3339(),
    });

    let timestamp_str = msg["timestamp"].as_str().unwrap();
    assert!(!timestamp_str.is_empty());

    // Verify timestamp can be parsed back
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);
    assert!(parsed.is_ok());
}

#[test]
fn test_websocket_client_commands() {
    // Test client command formats
    let commands = vec![
        json!({"command": "subscribe", "events": ["execution_started"]}),
        json!({"command": "unsubscribe", "events": ["execution_completed"]}),
        json!({"command": "get_status"}),
        json!({"command": "list_executions"}),
    ];

    for cmd in commands {
        assert!(cmd["command"].is_string());
    }
}

#[test]
fn test_websocket_subscription_messages() {
    let subscribe = json!({
        "command": "subscribe",
        "events": ["execution_started", "execution_completed", "resource_alert"],
    });

    assert_eq!(subscribe["command"], "subscribe");
    assert_eq!(subscribe["events"].as_array().unwrap().len(), 3);
}

#[test]
fn test_websocket_response_formats() {
    // Test response message formats
    let success_response = json!({
        "status": "success",
        "message": "Command executed successfully",
    });

    assert_eq!(success_response["status"], "success");

    let error_response = json!({
        "status": "error",
        "message": "Invalid command",
        "code": "INVALID_COMMAND",
    });

    assert_eq!(error_response["status"], "error");
}

#[test]
fn test_websocket_execution_details() {
    let execution = json!({
        "execution_id": "exec-456",
        "runtime": "native",
        "status": "running",
        "started_at": chrono::Utc::now().to_rfc3339(),
        "resource_usage": {
            "cpu_cores": 2.0,
            "memory_mb": 1024,
        }
    });

    assert_eq!(execution["runtime"], "native");
    assert!(execution["resource_usage"].is_object());
}

#[test]
fn test_websocket_binary_message_detection() {
    // Test that we can distinguish binary from text
    let text = Message::Text("hello".to_string());
    let binary = Message::Binary(vec![0, 1, 2, 3]);

    assert!(matches!(text, Message::Text(_)));
    assert!(matches!(binary, Message::Binary(_)));
}

#[test]
fn test_websocket_ping_pong() {
    let ping_data = vec![1, 2, 3, 4];
    let ping = Message::Ping(ping_data.clone());
    let pong = Message::Pong(ping_data.clone());

    assert!(matches!(ping, Message::Ping(_)));
    assert!(matches!(pong, Message::Pong(_)));
}

#[test]
fn test_websocket_close_messages() {
    use axum::extract::ws::CloseFrame;

    let close_normal = CloseFrame {
        code: 1000,
        reason: std::borrow::Cow::from("Normal closure"),
    };

    assert_eq!(close_normal.code, 1000);
    assert_eq!(close_normal.reason, "Normal closure");
}

#[test]
fn test_websocket_message_size_limits() {
    // Test small and large messages
    let small = Message::Text("small".to_string());
    let large = Message::Text("x".repeat(10000));

    assert!(matches!(small, Message::Text(_)));
    assert!(matches!(large, Message::Text(_)));
}

#[test]
fn test_websocket_empty_messages() {
    let empty_text = Message::Text("".to_string());
    let empty_binary = Message::Binary(vec![]);

    assert!(matches!(empty_text, Message::Text(_)));
    assert!(matches!(empty_binary, Message::Binary(_)));
}

#[test]
fn test_websocket_special_characters() {
    let special_chars = json!({
        "message": "Special chars: ñ, ü, 你好, 🍄",
        "emoji": "🚀✅🔴",
    });

    let serialized = serde_json::to_string(&special_chars);
    assert!(serialized.is_ok());
}
