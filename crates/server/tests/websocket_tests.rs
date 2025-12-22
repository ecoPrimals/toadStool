//! Tests for WebSocket handlers

use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::config::ServerConfig;
use toadstool_server::state::{
    ActiveExecution, ClientInfo, ServerEvent, ServerState, ServerStatistics,
};
use toadstool_testing::mocks::MockResourceMonitor;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

/// Helper to create a test ServerState
fn create_test_state() -> ServerState {
    let (event_tx, _) = broadcast::channel(100);
    let config = ServerConfig::default();
    let resource_monitor = Arc::new(MockResourceMonitor::new());
    let stats = Arc::new(RwLock::new(ServerStatistics::default()));

    ServerState {
        runtime_engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
        active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        event_broadcaster: event_tx,
        config,
        resource_monitor,
        stats,
        capability_provider: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_execution_started_event() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let execution_id = Uuid::new_v4();
    let event = ServerEvent::ExecutionStarted {
        execution_id,
        runtime_type: RuntimeType::Native,
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "execution_started");
    assert!(parsed["data"]["execution_id"].is_string());
    assert!(parsed["data"]["runtime_type"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_execution_completed_event() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let execution_id = Uuid::new_v4();
    let event = ServerEvent::ExecutionCompleted {
        execution_id,
        status: ExecutionStatus::Success,
        duration_ms: 1500,
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "execution_completed");
    assert!(parsed["data"]["execution_id"].is_string());
    assert!(parsed["data"]["status"].is_string());
    assert_eq!(parsed["data"]["duration_ms"], 1500);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_runtime_engine_registered_event() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: RuntimeType::Wasm,
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "runtime_engine_registered");
    assert!(parsed["data"]["runtime_type"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_resource_usage_update_event() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 45.5,
        memory_usage_percent: 62.3,
        active_executions: 5,
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "resource_usage_update");
    assert_eq!(parsed["data"]["cpu_usage_percent"], 45.5);
    assert_eq!(parsed["data"]["memory_usage_percent"], 62.3);
    assert_eq!(parsed["data"]["active_executions"], 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_health_status_changed_event() {
    use toadstool_server::websocket::format_server_event;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_error_occurred_event() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let execution_id = Uuid::new_v4();
    let event = ServerEvent::ErrorOccurred {
        error_type: "RuntimeError".to_string(),
        message: "Execution failed".to_string(),
        execution_id: Some(execution_id),
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "error_occurred");
    assert_eq!(parsed["data"]["error_type"], "RuntimeError");
    assert_eq!(parsed["data"]["message"], "Execution failed");
    assert!(parsed["data"]["execution_id"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_error_occurred_event_without_execution_id() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let event = ServerEvent::ErrorOccurred {
        error_type: "SystemError".to_string(),
        message: "System overload".to_string(),
        execution_id: None,
        timestamp,
    };

    let formatted = format_server_event(&event);
    let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

    assert_eq!(parsed["type"], "error_occurred");
    assert_eq!(parsed["data"]["error_type"], "SystemError");
    assert_eq!(parsed["data"]["execution_id"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_ping_message() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let ping_message = json!({
        "type": "ping"
    })
    .to_string();

    let result = handle_client_message(&ping_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "pong");
        assert!(parsed["timestamp"].is_string());
    } else {
        panic!("Expected text message response");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_get_status_message() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let status_message = json!({
        "type": "get_status"
    })
    .to_string();

    let result = handle_client_message(&status_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "status");
        assert!(parsed["data"]["active_executions"].is_number());
        assert!(parsed["data"]["runtime_engines"].is_number());
        assert!(parsed["data"]["timestamp"].is_string());
    } else {
        panic!("Expected text message response");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_subscribe_message() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let subscribe_message = json!({
        "type": "subscribe"
    })
    .to_string();

    let result = handle_client_message(&subscribe_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "subscribed");
        assert_eq!(parsed["message"], "Subscribed to server events");
    } else {
        panic!("Expected text message response");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_unknown_message_type() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let unknown_message = json!({
        "type": "unknown_command"
    })
    .to_string();

    let result = handle_client_message(&unknown_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "Unknown message type");
    } else {
        panic!("Expected text message response");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_message_without_type() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let invalid_message = json!({
        "data": "some data"
    })
    .to_string();

    let result = handle_client_message(&invalid_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["type"], "error");
        assert_eq!(parsed["message"], "Unknown message type");
    } else {
        panic!("Expected text message response");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_invalid_json_message() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, _rx) = mpsc::unbounded_channel::<Message>();

    let invalid_json = "{ not valid json }";

    let result = handle_client_message(invalid_json, &tx, &state).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_creation() {
    let state = create_test_state();

    // Verify initial state
    assert_eq!(state.active_executions.read().await.len(), 0);
    assert_eq!(state.runtime_engines.read().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_event_broadcasting() {
    let state = create_test_state();
    let mut receiver = state.event_broadcaster.subscribe();

    let test_execution_id = Uuid::new_v4();
    let test_event = ServerEvent::ExecutionStarted {
        execution_id: test_execution_id,
        runtime_type: RuntimeType::Native,
        timestamp: Utc::now(),
    };

    // Send event
    let _ = state.event_broadcaster.send(test_event.clone());

    // Receive event
    let received = receiver.recv().await.unwrap();

    match received {
        ServerEvent::ExecutionStarted {
            execution_id,
            runtime_type,
            ..
        } => {
            assert_eq!(execution_id, test_execution_id);
            assert!(matches!(runtime_type, RuntimeType::Native));
        }
        _ => panic!("Expected ExecutionStarted event"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_event_subscribers() {
    let state = create_test_state();
    let mut receiver1 = state.event_broadcaster.subscribe();
    let mut receiver2 = state.event_broadcaster.subscribe();

    let test_event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: RuntimeType::Wasm,
        timestamp: Utc::now(),
    };

    // Send event
    let _ = state.event_broadcaster.send(test_event);

    // Both receivers should get the event
    let received1 = receiver1.recv().await.unwrap();
    let received2 = receiver2.recv().await.unwrap();

    assert!(matches!(
        received1,
        ServerEvent::RuntimeEngineRegistered { .. }
    ));
    assert!(matches!(
        received2,
        ServerEvent::RuntimeEngineRegistered { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_message_handling() {
    use axum::extract::ws::Message;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Send multiple messages concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let tx = tx.clone();
        let state = state.clone();
        let handle = tokio::spawn(async move {
            let message = json!({
                "type": "ping",
                "id": i
            })
            .to_string();
            // Ignore the error for Send requirement
            let _ = handle_client_message(&message, &tx, &state).await;
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Should have received 10 pong responses
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_format_all_event_types() {
    use toadstool_server::websocket::format_server_event;

    let timestamp = Utc::now();
    let exec_id = Uuid::new_v4();

    let events = vec![
        ServerEvent::ExecutionStarted {
            execution_id: exec_id,
            runtime_type: RuntimeType::Native,
            timestamp,
        },
        ServerEvent::ExecutionCompleted {
            execution_id: exec_id,
            status: ExecutionStatus::Success,
            duration_ms: 100,
            timestamp,
        },
        ServerEvent::RuntimeEngineRegistered {
            runtime_type: RuntimeType::Wasm,
            timestamp,
        },
        ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            active_executions: 3,
            timestamp,
        },
        ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "OK".to_string(),
            timestamp,
        },
        ServerEvent::ErrorOccurred {
            error_type: "Test".to_string(),
            message: "Test error".to_string(),
            execution_id: None,
            timestamp,
        },
    ];

    // All events should format successfully
    for event in events {
        let formatted = format_server_event(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();
        assert!(parsed["type"].is_string());
        assert!(parsed["data"].is_object());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_status_response_structure() {
    use axum::extract::ws::Message;
    use std::time::Duration;
    use toadstool_server::websocket::handle_client_message;

    let state = create_test_state();

    // Add some test data to state
    let exec_id = Uuid::new_v4();
    let execution = ActiveExecution {
        execution_id: exec_id,
        runtime_type: RuntimeType::Native,
        started_at: Utc::now(),
        timeout: Duration::from_secs(300),
        status: ExecutionStatus::Running,
        client_info: ClientInfo {
            ip_address: None,
            user_agent: None,
            api_key: None,
            authenticated_user: None,
        },
    };
    state
        .active_executions
        .write()
        .await
        .insert(exec_id, execution);

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let status_message = json!({
        "type": "get_status"
    })
    .to_string();

    let result = handle_client_message(&status_message, &tx, &state).await;
    assert!(result.is_ok());

    if let Some(Message::Text(response)) = rx.recv().await {
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["data"]["active_executions"], 1);
        // Runtime engines count will be 0 since we didn't add any
        assert!(parsed["data"]["runtime_engines"].is_number());
    } else {
        panic!("Expected text message response");
    }
}
