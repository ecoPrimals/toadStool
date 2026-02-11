//! WebSocket handlers for real-time communication

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::state::{ServerEvent, ServerState};

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    debug!("WebSocket upgrade requested");
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: ServerState) {
    info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut event_receiver = state.event_broadcaster.subscribe();

    // Create a channel for sending messages to the WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<axum::extract::ws::Message>();

    // Send welcome message
    if let Err(e) = tx.send(axum::extract::ws::Message::Text(
        json!({
            "type": "welcome",
            "message": "Connected to ToadStool Server",
            "timestamp": chrono::Utc::now(),
        })
        .to_string(),
    )) {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    // Task to send messages to WebSocket
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = sender.send(msg).await {
                error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    });

    // Task to handle incoming messages and server events
    let handler_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(axum::extract::ws::Message::Text(text))) => {
                            debug!("Received WebSocket message: {}", text);
                            if let Err(e) = handle_client_message(&text, &tx, &state).await {
                                error!("Failed to handle client message: {}", e);
                            }
                        }
                        Some(Ok(axum::extract::ws::Message::Close(_))) => {
                            info!("WebSocket connection closed by client");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            debug!("WebSocket receiver closed");
                            break;
                        }
                        _ => {}
                    }
                }

                // Handle outgoing server events
                event = event_receiver.recv() => {
                    match event {
                        Ok(event) => {
                            let message = format_server_event(&event);
                            if let Err(e) = tx.send(axum::extract::ws::Message::Text(message)) {
                                error!("Failed to send server event: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            debug!("Event receiver error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = sender_task => {
            debug!("WebSocket sender task completed");
        }
        _ = handler_task => {
            debug!("WebSocket handler task completed");
        }
    }

    info!("WebSocket connection closed");
}

/// Handle client message
pub async fn handle_client_message(
    message: &str,
    tx: &mpsc::UnboundedSender<axum::extract::ws::Message>,
    state: &ServerState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request: serde_json::Value = serde_json::from_slice(message.as_bytes())?;

    match request.get("type").and_then(|t| t.as_str()) {
        Some("ping") => {
            let response = json!({
                "type": "pong",
                "timestamp": chrono::Utc::now(),
            });
            tx.send(axum::extract::ws::Message::Text(response.to_string()))?;
        }
        Some("get_status") => {
            let response = json!({
                "type": "status",
                "data": {
                    "active_executions": state.active_executions.read().await.len(),
                    "runtime_engines": state.runtime_engines.read().await.len(),
                    "timestamp": chrono::Utc::now(),
                }
            });
            tx.send(axum::extract::ws::Message::Text(response.to_string()))?;
        }
        Some("subscribe") => {
            let response = json!({
                "type": "subscribed",
                "message": "Subscribed to server events",
                "timestamp": chrono::Utc::now(),
            });
            tx.send(axum::extract::ws::Message::Text(response.to_string()))?;
        }
        _ => {
            let response = json!({
                "type": "error",
                "message": "Unknown message type",
                "timestamp": chrono::Utc::now(),
            });
            tx.send(axum::extract::ws::Message::Text(response.to_string()))?;
        }
    }

    Ok(())
}

/// Format server event for WebSocket transmission
pub fn format_server_event(event: &ServerEvent) -> String {
    match event {
        ServerEvent::ExecutionStarted {
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
        ServerEvent::ExecutionCompleted {
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
        ServerEvent::RuntimeEngineRegistered {
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
        ServerEvent::ResourceUsageUpdate {
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
        ServerEvent::HealthStatusChanged {
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
        ServerEvent::ErrorOccurred {
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
