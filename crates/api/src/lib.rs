//! ToadStool Advanced API & Interface Layer
//!
//! This module provides comprehensive API and interface capabilities including:
//! - RESTful API endpoints for execution and monitoring
//! - WebSocket real-time communication
//! - Command-line tools and management interfaces
//! - Web-based monitoring dashboard
//! - Integration APIs for external tools

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use async_trait::async_trait;
use axum::{
    extract::{State, Path, Query, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post, delete},
    Json, Router,
};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error};
use uuid::Uuid;

use toadstool::{RuntimeType, ToadStoolResult, ToadStoolError};
use toadstool_common::*;
use toadstool_management_monitoring::SystemResourceMonitor;
use toadstool_management_analytics::AnalyticsEngine;
use toadstool_distributed::DistributedCoordinator;

pub mod byob;
pub use byob::*;

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_address: String,
    pub enable_rest: bool,
    pub enable_websocket: bool,
    pub cors_enabled: bool,
}

/// API server state
#[derive(Clone)]
pub struct ApiState {
    pub event_broadcaster: broadcast::Sender<ApiEvent>,
    pub executions: Arc<tokio::sync::RwLock<HashMap<Uuid, ExecutionInfo>>>,
}

/// API events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiEvent {
    ExecutionStarted { execution_id: Uuid },
    ExecutionCompleted { execution_id: Uuid, success: bool },
    ClusterNodeAdded { node_id: String },
    AlertTriggered { message: String },
}

/// Execution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub execution_id: Uuid,
    pub status: String,
    pub runtime_type: RuntimeType,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// REST API request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub workload_type: String,
    pub runtime_type: RuntimeType,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub execution_id: Uuid,
    pub status: String,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterStatusResponse {
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub cluster_load: f64,
    pub active_executions: u32,
}

/// REST API handlers
pub async fn submit_execution(
    State(state): State<ApiState>,
    Json(request): Json<ExecutionRequest>,
) -> Result<Json<ExecutionResponse>, StatusCode> {
    info!("Received execution request for workload type: {}", request.workload_type);
    
    let execution_id = Uuid::new_v4();
    let execution_info = ExecutionInfo {
        execution_id,
        status: "submitted".to_string(),
        runtime_type: request.runtime_type,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
    };
    
    // Store execution info
    state.executions.write().await.insert(execution_id, execution_info);
    
    // Broadcast event
    let _ = state.event_broadcaster.send(ApiEvent::ExecutionStarted { execution_id });
    
    Ok(Json(ExecutionResponse {
        execution_id,
        status: "submitted".to_string(),
        submitted_at: Utc::now(),
    }))
}

pub async fn get_execution_status(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<ExecutionInfo>, StatusCode> {
    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(info) => Ok(Json(info.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn cancel_execution(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut executions = state.executions.write().await;
    match executions.get_mut(&execution_id) {
        Some(info) => {
            info.status = "cancelled".to_string();
            info.completed_at = Some(Utc::now());
            Ok(Json(serde_json::json!({ "message": "Execution cancelled" })))
        },
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn get_cluster_status() -> Result<Json<ClusterStatusResponse>, StatusCode> {
    Ok(Json(ClusterStatusResponse {
        total_nodes: 3,
        healthy_nodes: 3,
        cluster_load: 45.5,
        active_executions: 12,
    }))
}

/// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: ApiState) {
    info!("WebSocket connection established");
    
    let mut receiver = state.event_broadcaster.subscribe();
    let (mut sender, _receiver) = socket.split();
    
    // Send events to WebSocket clients
    while let Ok(event) = receiver.recv().await {
        let event_json = serde_json::to_string(&event).unwrap_or_default();
        if sender.send(axum::extract::ws::Message::Text(event_json)).await.is_err() {
            break;
        }
    }
    
    info!("WebSocket connection closed");
}

/// Dashboard HTML
pub async fn serve_dashboard() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>ToadStool Dashboard</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .dashboard { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
        .panel { border: 1px solid #ccc; padding: 15px; border-radius: 5px; }
        .status { font-weight: bold; color: green; }
        #events { height: 200px; overflow-y: auto; border: 1px solid #ddd; padding: 10px; }
    </style>
</head>
<body>
    <h1>ToadStool Monitoring Dashboard</h1>
    
    <div class="dashboard">
        <div class="panel">
            <h3>Cluster Status</h3>
            <p>Total Nodes: <span class="status">3</span></p>
            <p>Healthy Nodes: <span class="status">3</span></p>
            <p>Cluster Load: <span class="status">45.5%</span></p>
            <p>Active Executions: <span class="status">12</span></p>
        </div>
        
        <div class="panel">
            <h3>Recent Executions</h3>
            <ul id="executions">
                <li>Execution abc123 - Completed</li>
                <li>Execution def456 - Running</li>
                <li>Execution ghi789 - Pending</li>
            </ul>
        </div>
        
        <div class="panel">
            <h3>Analytics</h3>
            <p>Average Execution Time: <span class="status">2.3s</span></p>
            <p>Success Rate: <span class="status">98.5%</span></p>
            <p>Throughput: <span class="status">1200/hour</span></p>
        </div>
        
        <div class="panel">
            <h3>Real-time Events</h3>
            <div id="events"></div>
        </div>
    </div>
    
    <script>
        // WebSocket connection for real-time updates
        const ws = new WebSocket('ws://localhost:8080/ws');
        const eventsDiv = document.getElementById('events');
        
        ws.onmessage = function(event) {
            const eventData = JSON.parse(event.data);
            const eventElement = document.createElement('div');
            eventElement.textContent = new Date().toLocaleTimeString() + ': ' + JSON.stringify(eventData);
            eventsDiv.appendChild(eventElement);
            eventsDiv.scrollTop = eventsDiv.scrollHeight;
        };
        
        ws.onerror = function(error) {
            console.error('WebSocket error:', error);
        };
    </script>
</body>
</html>
    "#)
}

/// Advanced API server
pub struct AdvancedApiServer {
    config: ApiConfig,
    state: ApiState,
}

impl AdvancedApiServer {
    /// Create a new API server
    pub fn new(config: ApiConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        let state = ApiState {
            event_broadcaster: event_sender,
            executions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };
        
        Self { config, state }
    }
    
    /// Start the API server
    pub async fn start(&self) -> ToadStoolResult<()> {
        info!("Starting API server on {}", self.config.bind_address);
        
        let mut app = Router::new();
        
        // REST API routes
        if self.config.enable_rest {
            app = app
                .route("/api/v1/executions", post(submit_execution))
                .route("/api/v1/executions/:execution_id", get(get_execution_status))
                .route("/api/v1/executions/:execution_id", delete(cancel_execution))
                .route("/api/v1/cluster/status", get(get_cluster_status));
        }
        
        // WebSocket endpoint
        if self.config.enable_websocket {
            app = app.route("/ws", get(websocket_handler));
        }
        
        // Dashboard
        app = app
            .route("/", get(serve_dashboard))
            .route("/dashboard", get(serve_dashboard));
        
        // Add CORS if enabled
        if self.config.cors_enabled {
            app = app.layer(CorsLayer::permissive());
        }
        
        // Add state and convert to Router<()>
        let app = app.with_state(self.state.clone());
        
        // Start server
        let addr: SocketAddr = self.config.bind_address.parse()
            .map_err(|e| ToadStoolError::configuration(format!("Invalid bind address: {}", e)))?;
        
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| ToadStoolError::network(format!("Failed to bind: {}", e)))?;
        
        info!("API server listening on {}", addr);
        info!("Dashboard: http://{}/dashboard", addr);
        
        // Serve the application
        axum::serve(listener, app)
            .await
            .map_err(|e| ToadStoolError::network(format!("Server error: {}", e)))?;
        
        Ok(())
    }
    
    /// Send an event to all connected WebSocket clients
    pub fn broadcast_event(&self, event: ApiEvent) {
        let _ = self.state.event_broadcaster.send(event);
    }
}

/// Command-line interface
#[derive(Parser)]
#[command(name = "toadstool")]
#[command(about = "ToadStool Universal Compute Platform CLI")]
pub struct ToadStoolCli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Start the API server
    Server {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
    },
    /// Submit an execution request
    Execute {
        #[arg(short, long)]
        workload: String,
        #[arg(short, long, default_value = "native")]
        runtime: String,
    },
    /// Get execution status
    Status {
        execution_id: String,
    },
    /// Monitor cluster health
    Monitor,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            enable_rest: true,
            enable_websocket: true,
            cors_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1:8080");
        assert!(config.enable_rest);
        assert!(config.enable_websocket);
        assert!(config.cors_enabled);
    }
    
    #[test]
    fn test_execution_info_serialization() {
        let info = ExecutionInfo {
            execution_id: Uuid::new_v4(),
            status: "running".to_string(),
            runtime_type: RuntimeType::Native,
            submitted_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };
        
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("execution_id"));
        assert!(json.contains("running"));
    }
} 