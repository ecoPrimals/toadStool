//! Modern WebSocket handler with structured event handling

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{types::ApiEvent, ApiState};

/// WebSocket connection information
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub id: Uuid,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub subscriptions: Vec<String>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// Client subscribes to event types
    Subscribe { event_types: Vec<String> },
    /// Client unsubscribes from event types
    Unsubscribe { event_types: Vec<String> },
    /// Ping message for keepalive
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Pong response
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Event broadcast to client
    Event { event: ApiEvent },
    /// Error message
    Error { message: String, code: String },
    /// Connection acknowledgment
    Connected { connection_id: Uuid },
}

/// WebSocket connection manager
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnection>>>,
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, connection: WebSocketConnection) {
        let mut connections = self.connections.write().await;
        connections.insert(connection.id, connection);
    }

    pub async fn remove_connection(&self, connection_id: &Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
    }

    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    pub async fn broadcast_event(&self, event: &ApiEvent) {
        let connections = self.connections.read().await;
        debug!("Broadcasting event to {} connections", connections.len());

        // In a real implementation, you would send the event to each connection
        // For now, we just log it
        for (id, _conn) in connections.iter() {
            debug!("Would send event to connection {}: {:?}", id, event);
        }
    }
}

/// Modern WebSocket handler with connection management
pub async fn handle_websocket(socket: WebSocket, state: ApiState) {
    let connection_id = Uuid::new_v4();
    let _connection = WebSocketConnection {
        id: connection_id,
        connected_at: chrono::Utc::now(),
        last_ping: None,
        subscriptions: Vec::new(),
    };

    info!("WebSocket connection established: {}", connection_id);

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Send connection acknowledgment
    let ack_message = WebSocketMessage::Connected { connection_id };
    if let Ok(msg_json) = serde_json::to_string(&ack_message) {
        if sender.send(Message::Text(msg_json)).await.is_err() {
            error!("Failed to send connection acknowledgment");
            return;
        }
    }

    // Subscribe to events
    let mut event_receiver = state.event_broadcaster.subscribe();

    // Handle incoming messages
    let sender_clone = Arc::new(tokio::sync::Mutex::new(sender));
    let sender_for_events = sender_clone.clone();
    let sender_for_keepalive = sender_clone.clone();

    // Task to handle incoming messages from client
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received WebSocket message: {}", text);

                    // Parse message
                    match serde_json::from_str::<WebSocketMessage>(&text) {
                        Ok(ws_msg) => {
                            match ws_msg {
                                WebSocketMessage::Subscribe { event_types } => {
                                    info!(
                                        "Client {} subscribed to: {:?}",
                                        connection_id, event_types
                                    );

                                    // Update connection subscriptions
                                    let mut connections =
                                        state.websocket_manager.connections.write().await;
                                    if let Some(connection) = connections.get_mut(&connection_id) {
                                        for event_type in event_types {
                                            if !connection.subscriptions.contains(&event_type) {
                                                connection.subscriptions.push(event_type);
                                            }
                                        }
                                        debug!(
                                            "Updated subscriptions for client {}: {:?}",
                                            connection_id, connection.subscriptions
                                        );
                                    }
                                }
                                WebSocketMessage::Unsubscribe { event_types } => {
                                    info!(
                                        "Client {} unsubscribed from: {:?}",
                                        connection_id, event_types
                                    );

                                    // Update connection subscriptions
                                    let mut connections =
                                        state.websocket_manager.connections.write().await;
                                    if let Some(connection) = connections.get_mut(&connection_id) {
                                        for event_type in &event_types {
                                            connection
                                                .subscriptions
                                                .retain(|sub| sub != event_type);
                                        }
                                        debug!(
                                            "Updated subscriptions for client {}: {:?}",
                                            connection_id, connection.subscriptions
                                        );
                                    }
                                }
                                WebSocketMessage::Ping { timestamp } => {
                                    debug!("Received ping from client {}", connection_id);
                                    let pong = WebSocketMessage::Pong { timestamp };
                                    if let Ok(pong_json) = serde_json::to_string(&pong) {
                                        let mut sender = sender_clone.lock().await;
                                        if sender.send(Message::Text(pong_json)).await.is_err() {
                                            error!(
                                                "Failed to send pong to client {}",
                                                connection_id
                                            );
                                            break;
                                        }
                                    }
                                }
                                _ => {
                                    warn!("Unexpected message type from client {}", connection_id);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse WebSocket message: {}", e);
                            let error_msg = WebSocketMessage::Error {
                                message: "Invalid message format".to_string(),
                                code: "INVALID_MESSAGE".to_string(),
                            };
                            if let Ok(error_json) = serde_json::to_string(&error_msg) {
                                let mut sender = sender_clone.lock().await;
                                if sender.send(Message::Text(error_json)).await.is_err() {
                                    error!(
                                        "Failed to send error message to client {}",
                                        connection_id
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!(
                        "Received binary message from client {} (not supported)",
                        connection_id
                    );
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} initiated close", connection_id);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping from client {}", connection_id);
                    let mut sender = sender_clone.lock().await;
                    if sender.send(Message::Pong(data)).await.is_err() {
                        error!("Failed to send pong to client {}", connection_id);
                        break;
                    }
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong from client {}", connection_id);
                }
                Err(e) => {
                    error!("WebSocket error for client {}: {}", connection_id, e);
                    break;
                }
            }
        }
    });

    // Task to handle outgoing events
    let outgoing_task = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            debug!(
                "Broadcasting event to client {}: {:?}",
                connection_id, event
            );

            let ws_message = WebSocketMessage::Event { event };
            if let Ok(msg_json) = serde_json::to_string(&ws_message) {
                let mut sender = sender_for_events.lock().await;
                if sender.send(Message::Text(msg_json)).await.is_err() {
                    error!("Failed to send event to client {}", connection_id);
                    break;
                }
            }
        }
    });

    // Keepalive task
    let keepalive_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let ping_msg = WebSocketMessage::Ping {
                timestamp: chrono::Utc::now(),
            };
            if let Ok(ping_json) = serde_json::to_string(&ping_msg) {
                let mut sender = sender_for_keepalive.lock().await;
                if sender.send(Message::Text(ping_json)).await.is_err() {
                    debug!("Keepalive failed for client {}", connection_id);
                    break;
                }
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = incoming_task => {
            debug!("Incoming task completed for client {}", connection_id);
        }
        _ = outgoing_task => {
            debug!("Outgoing task completed for client {}", connection_id);
        }
        _ = keepalive_task => {
            debug!("Keepalive task completed for client {}", connection_id);
        }
    }

    info!("WebSocket connection closed: {}", connection_id);
}
