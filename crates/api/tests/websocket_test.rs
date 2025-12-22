//! Comprehensive tests for WebSocket handling
//!
//! Coverage target: 3% → 40% (25 tests)

use uuid::Uuid;

use toadstool_api::types::*;
use toadstool_api::websocket::*;

// ============================================================================
// WebSocketConnection Tests (5 tests)
// ============================================================================

#[test]
fn test_websocket_connection_creation() {
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: Vec::new(),
    };

    assert!(conn.subscriptions.is_empty());
    assert!(conn.last_ping.is_none());
}

#[test]
fn test_websocket_connection_with_subscriptions() {
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: vec![
            "execution_started".to_string(),
            "execution_completed".to_string(),
        ],
    };

    assert_eq!(conn.subscriptions.len(), 2);
    assert!(conn
        .subscriptions
        .contains(&"execution_started".to_string()));
}

#[test]
fn test_websocket_connection_clone() {
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: chrono::Utc::now(),
        last_ping: Some(chrono::Utc::now()),
        subscriptions: vec!["test".to_string()],
    };

    let cloned = conn.clone();
    assert_eq!(conn.id, cloned.id);
    assert_eq!(conn.subscriptions, cloned.subscriptions);
}

#[test]
fn test_websocket_connection_debug() {
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: vec![],
    };

    let debug_str = format!("{:?}", conn);
    assert!(debug_str.contains("WebSocketConnection"));
}

#[test]
fn test_websocket_connection_with_last_ping() {
    let now = chrono::Utc::now();
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: now,
        last_ping: Some(now),
        subscriptions: vec![],
    };

    assert!(conn.last_ping.is_some());
    assert_eq!(conn.last_ping.unwrap(), now);
}

// ============================================================================
// WebSocketMessage Tests (8 tests)
// ============================================================================

#[test]
fn test_websocket_message_subscribe_serialization() {
    let msg = WebSocketMessage::Subscribe {
        event_types: vec!["execution_started".to_string()],
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("Subscribe"));
    assert!(json_str.contains("execution_started"));
}

#[test]
fn test_websocket_message_subscribe_deserialization() {
    let json_str = r#"{"type":"Subscribe","data":{"event_types":["execution_started"]}}"#;
    let msg: WebSocketMessage = serde_json::from_str(json_str).unwrap();

    match msg {
        WebSocketMessage::Subscribe { event_types } => {
            assert_eq!(event_types.len(), 1);
            assert_eq!(event_types[0], "execution_started");
        }
        _ => panic!("Expected Subscribe message"),
    }
}

#[test]
fn test_websocket_message_unsubscribe() {
    let msg = WebSocketMessage::Unsubscribe {
        event_types: vec!["test".to_string()],
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: WebSocketMessage = serde_json::from_str(&json_str).unwrap();

    match parsed {
        WebSocketMessage::Unsubscribe { event_types } => {
            assert_eq!(event_types[0], "test");
        }
        _ => panic!("Expected Unsubscribe message"),
    }
}

#[test]
fn test_websocket_message_ping() {
    let now = chrono::Utc::now();
    let msg = WebSocketMessage::Ping { timestamp: now };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("Ping"));
}

#[test]
fn test_websocket_message_pong() {
    let now = chrono::Utc::now();
    let msg = WebSocketMessage::Pong { timestamp: now };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: WebSocketMessage = serde_json::from_str(&json_str).unwrap();

    match parsed {
        WebSocketMessage::Pong { timestamp } => {
            // Timestamps should be approximately equal
            assert!((timestamp.timestamp() - now.timestamp()).abs() < 2);
        }
        _ => panic!("Expected Pong message"),
    }
}

#[test]
fn test_websocket_message_error() {
    let msg = WebSocketMessage::Error {
        message: "Test error".to_string(),
        code: "TEST_ERROR".to_string(),
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: WebSocketMessage = serde_json::from_str(&json_str).unwrap();

    match parsed {
        WebSocketMessage::Error { message, code } => {
            assert_eq!(message, "Test error");
            assert_eq!(code, "TEST_ERROR");
        }
        _ => panic!("Expected Error message"),
    }
}

#[test]
fn test_websocket_message_connected() {
    let conn_id = Uuid::new_v4();
    let msg = WebSocketMessage::Connected {
        connection_id: conn_id,
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: WebSocketMessage = serde_json::from_str(&json_str).unwrap();

    match parsed {
        WebSocketMessage::Connected { connection_id } => {
            assert_eq!(connection_id, conn_id);
        }
        _ => panic!("Expected Connected message"),
    }
}

#[test]
fn test_websocket_message_event() {
    let event = ApiEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: toadstool::RuntimeType::Native,
        timestamp: chrono::Utc::now(),
    };

    let msg = WebSocketMessage::Event { event };

    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("ExecutionStarted"));
}

// ============================================================================
// WebSocketManager Tests (7 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_creation() {
    let manager = WebSocketManager::new();
    let count = manager.get_connection_count().await;
    assert_eq!(count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_default() {
    let manager = WebSocketManager::default();
    let count = manager.get_connection_count().await;
    assert_eq!(count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_add_connection() {
    let manager = WebSocketManager::new();
    let conn = WebSocketConnection {
        id: Uuid::new_v4(),
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: vec![],
    };

    manager.add_connection(conn).await;
    let count = manager.get_connection_count().await;
    assert_eq!(count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_remove_connection() {
    let manager = WebSocketManager::new();
    let conn_id = Uuid::new_v4();
    let conn = WebSocketConnection {
        id: conn_id,
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: vec![],
    };

    manager.add_connection(conn).await;
    assert_eq!(manager.get_connection_count().await, 1);

    manager.remove_connection(&conn_id).await;
    assert_eq!(manager.get_connection_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_multiple_connections() {
    let manager = WebSocketManager::new();

    for _ in 0..5 {
        let conn = WebSocketConnection {
            id: Uuid::new_v4(),
            connected_at: chrono::Utc::now(),
            last_ping: None,
            subscriptions: vec![],
        };
        manager.add_connection(conn).await;
    }

    assert_eq!(manager.get_connection_count().await, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_broadcast_event() {
    let manager = WebSocketManager::new();

    // Add connections
    for _ in 0..3 {
        let conn = WebSocketConnection {
            id: Uuid::new_v4(),
            connected_at: chrono::Utc::now(),
            last_ping: None,
            subscriptions: vec!["test_event".to_string()],
        };
        manager.add_connection(conn).await;
    }

    let event = ApiEvent::ExecutionStarted {
        execution_id: Uuid::new_v4(),
        runtime_type: toadstool::RuntimeType::Native,
        timestamp: chrono::Utc::now(),
    };

    // Should not panic
    manager.broadcast_event(&event).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_websocket_manager_concurrent_operations() {
    let manager = Arc::new(WebSocketManager::new());

    // Spawn multiple tasks that add connections
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let conn = WebSocketConnection {
                id: Uuid::new_v4(),
                connected_at: chrono::Utc::now(),
                last_ping: None,
                subscriptions: vec![],
            };
            manager_clone.add_connection(conn).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(manager.get_connection_count().await, 10);
}

// ============================================================================
// Message Parsing Tests (5 tests)
// ============================================================================

#[test]
fn test_parse_subscribe_message() {
    let json = r#"{"type":"Subscribe","data":{"event_types":["exec_started","exec_completed"]}}"#;
    let msg: Result<WebSocketMessage, _> = serde_json::from_str(json);

    assert!(msg.is_ok());
    match msg.unwrap() {
        WebSocketMessage::Subscribe { event_types } => {
            assert_eq!(event_types.len(), 2);
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_parse_invalid_message() {
    let json = r#"{"invalid": "json"}"#;
    let msg: Result<WebSocketMessage, _> = serde_json::from_str(json);

    assert!(msg.is_err());
}

#[test]
fn test_parse_malformed_json() {
    let json = r#"{"type":"Subscribe","data":}"#;
    let msg: Result<WebSocketMessage, _> = serde_json::from_str(json);

    assert!(msg.is_err());
}

#[test]
fn test_parse_empty_string() {
    let json = "";
    let msg: Result<WebSocketMessage, _> = serde_json::from_str(json);

    assert!(msg.is_err());
}

#[test]
fn test_serialize_then_deserialize() {
    let original = WebSocketMessage::Subscribe {
        event_types: vec!["test1".to_string(), "test2".to_string()],
    };

    let json = serde_json::to_string(&original).unwrap();
    let parsed: WebSocketMessage = serde_json::from_str(&json).unwrap();

    match (original, parsed) {
        (
            WebSocketMessage::Subscribe {
                event_types: types1,
            },
            WebSocketMessage::Subscribe {
                event_types: types2,
            },
        ) => {
            assert_eq!(types1, types2);
        }
        _ => panic!("Message types don't match"),
    }
}

use std::sync::Arc;
