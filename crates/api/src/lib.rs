//! Modern API for ToadStool with OpenAPI support
//!
//! This crate provides a RESTful API with OpenAPI/Swagger documentation
//! for interacting with the ToadStool universal compute platform.

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// Public re-exports
pub use types::*;

// Internal modules
pub mod byob;
pub mod constants;
pub mod handlers;
pub mod middleware;
pub mod types;
pub mod websocket;

/// API server state
#[derive(Clone)]
pub struct ApiState {
    pub executions: Arc<RwLock<HashMap<Uuid, ExecutionInfo>>>,
    pub metrics: Arc<RwLock<ApiMetrics>>,
    pub event_broadcaster: broadcast::Sender<ApiEvent>,
    pub websocket_manager: Arc<websocket::WebSocketManager>,
}

/// API metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
        }
    }
}

/// Create the API router with all routes
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Execution management routes
        .route("/api/v2/executions", post(handlers::submit_execution))
        .route(
            "/api/v2/executions/:execution_id",
            get(handlers::get_execution_status),
        )
        .route("/api/v2/executions", get(handlers::list_executions))
        .route(
            "/api/v2/executions/:execution_id",
            axum::routing::delete(handlers::cancel_execution),
        )
        // Logs route
        .route(
            "/api/v2/executions/:execution_id/logs",
            get(handlers::get_execution_logs),
        )
        // Metrics routes
        .route(
            "/api/v2/executions/:execution_id/metrics",
            get(handlers::get_execution_metrics),
        )
        .route("/api/v2/metrics", get(handlers::get_api_metrics))
        // Cluster routes
        .route("/api/v2/cluster/status", get(handlers::get_cluster_status))
        // Health routes
        .route("/api/v2/health", get(handlers::health_check))
        // Workload routes
        .route("/api/v2/workload/execute", post(handlers::execute_workload))
        // Root route
        .route("/", get(root_handler))
        .with_state(state)
}

/// Root handler
async fn root_handler() -> (StatusCode, &'static str) {
    (
        StatusCode::OK,
        "ToadStool API v2 - Universal Compute Platform",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_metrics_default() {
        let metrics = ApiMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.average_response_time_ms, 0.0);
    }

    #[test]
    fn test_api_state_creation() {
        let executions = Arc::new(RwLock::new(HashMap::new()));
        let metrics = Arc::new(RwLock::new(ApiMetrics::default()));
        let (tx, _) = broadcast::channel(100);

        let state = ApiState {
            executions,
            metrics,
            event_broadcaster: tx,
            websocket_manager: Arc::new(websocket::WebSocketManager::new()),
        };

        assert_eq!(state.executions.try_read().unwrap().len(), 0);
    }
}
