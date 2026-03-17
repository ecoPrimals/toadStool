// SPDX-License-Identifier: AGPL-3.0-only
//! HTTP API server for ToadStool daemon mode
//!
//! Implements the REST API for workload submission and management.

#[cfg(feature = "daemon")]
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
};
#[cfg(feature = "daemon")]
use std::net::SocketAddr;
#[cfg(feature = "daemon")]
use std::sync::Arc;
#[cfg(feature = "daemon")]
use std::time::Instant;
#[cfg(feature = "daemon")]
use tower_http::cors::CorsLayer;
#[cfg(feature = "daemon")]
use tower_http::trace::TraceLayer;
#[cfg(feature = "daemon")]
use tracing::{error, info};

#[cfg(feature = "daemon")]
use super::api_types::*;
#[cfg(feature = "daemon")]
use super::workload_manager::WorkloadManager;

/// Shared server state
#[cfg(feature = "daemon")]
#[derive(Clone)]
pub struct ServerState {
    /// Server start time
    pub start_time: Instant,

    /// Workload manager
    pub workload_manager: Arc<WorkloadManager>,
}

/// Start HTTP API server
#[cfg(feature = "daemon")]
pub async fn start_http_server(
    port: u16,
    workload_manager: Arc<WorkloadManager>,
) -> crate::Result<()> {
    let state = ServerState {
        start_time: Instant::now(),
        workload_manager,
    };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("🌐 HTTP API server listening on {}", addr);
    info!("📊 Endpoints:");
    info!("   POST   /api/v1/workload/submit");
    info!("   GET    /api/v1/workload/:id");
    info!("   DELETE /api/v1/workload/:id");
    info!("   GET    /api/v1/workloads");
    info!("   GET    /health");
    info!("   GET    /metrics");

    // Use axum::serve for axum 0.7
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the axum router with all routes
#[cfg(feature = "daemon")]
pub fn create_router(state: ServerState) -> Router {
    Router::new()
        // Health and metrics
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        // Workload API (v1)
        .route("/api/v1/workload/submit", post(submit_workload_handler))
        .route("/api/v1/workload/:id", get(get_workload_handler))
        .route("/api/v1/workload/:id", delete(delete_workload_handler))
        .route("/api/v1/workloads", get(list_workloads_handler))
        // Add state
        .with_state(state)
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

// ============================================================================
// API Handlers
// ============================================================================

/// Health check handler
#[cfg(feature = "daemon")]
async fn health_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let active_workloads = state.workload_manager.active_workload_count().await;

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs,
        active_workloads,
        biomeos_connected: false, // Discovery via mDNS/environment
    })
}

/// Metrics handler (Prometheus-compatible)
#[cfg(feature = "daemon")]
async fn metrics_handler(State(state): State<ServerState>) -> impl IntoResponse {
    // Get all workload IDs
    let workload_ids = state.workload_manager.list_workloads().await;

    // Count by status
    let mut queued = 0;
    let mut running = 0;
    let mut completed = 0;
    let mut failed = 0;

    for id in workload_ids {
        if let Some(status_resp) = state.workload_manager.get_workload_status(&id).await {
            match status_resp.status {
                WorkloadStatus::Queued => queued += 1,
                WorkloadStatus::Running => running += 1,
                WorkloadStatus::Completed => completed += 1,
                WorkloadStatus::Failed => failed += 1,
                WorkloadStatus::Cancelled => {} // Don't count cancelled
            }
        }
    }

    let metrics = format!(
        "# HELP toadstool_daemon_uptime_seconds Daemon uptime in seconds\n\
         # TYPE toadstool_daemon_uptime_seconds counter\n\
         toadstool_daemon_uptime_seconds {}\n\
         \n\
         # HELP toadstool_workloads_total Total workloads by status\n\
         # TYPE toadstool_workloads_total gauge\n\
         toadstool_workloads_total{{status=\"queued\"}} {}\n\
         toadstool_workloads_total{{status=\"running\"}} {}\n\
         toadstool_workloads_total{{status=\"completed\"}} {}\n\
         toadstool_workloads_total{{status=\"failed\"}} {}\n\
         \n\
         # HELP toadstool_biomeos_connected biomeOS connection status (1=connected, 0=disconnected)\n\
         # TYPE toadstool_biomeos_connected gauge\n\
         toadstool_biomeos_connected {}\n",
        state.start_time.elapsed().as_secs(),
        queued,
        running,
        completed,
        failed,
        0 // Discovery via mDNS/environment (no hardcoded registry)
    );

    (StatusCode::OK, metrics)
}

/// Submit workload handler
#[cfg(feature = "daemon")]
async fn submit_workload_handler(
    State(state): State<ServerState>,
    Json(request): Json<SubmitWorkloadRequest>,
) -> Result<Json<SubmitWorkloadResponse>, ApiError> {
    info!(
        "📥 Received workload submission from: {}",
        request.requester
    );

    // Submit to workload manager
    let workload_id = state
        .workload_manager
        .submit_workload(request)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to submit workload: {e}")))?;

    info!("✅ Workload queued: {}", workload_id);

    Ok(Json(SubmitWorkloadResponse {
        workload_id,
        status: WorkloadStatus::Queued,
        message: "Workload queued successfully".to_string(),
    }))
}

/// Get workload status handler
#[cfg(feature = "daemon")]
async fn get_workload_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<WorkloadStatusResponse>, ApiError> {
    let status = state
        .workload_manager
        .get_workload_status(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Workload {id} not found")))?;

    Ok(Json(status))
}

/// Delete workload handler
#[cfg(feature = "daemon")]
async fn delete_workload_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    info!("🗑️  Cancelling workload: {}", id);

    state
        .workload_manager
        .cancel_workload(&id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Workload {id} not found: {e}")))?;

    info!("✅ Workload cancelled: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// List workloads handler
#[cfg(feature = "daemon")]
async fn list_workloads_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let workload_ids = state.workload_manager.list_workloads().await;

    let mut summaries = Vec::new();
    for id in &workload_ids {
        if let Some(status) = state.workload_manager.get_workload_status(id).await {
            let (requester, persistent) = state.workload_manager.get_workload_metadata(id).await;
            summaries.push(WorkloadSummary {
                workload_id: id.clone(),
                status: status.status,
                requester: requester.unwrap_or_else(|| "unknown".to_string()),
                started_at: status.started_at.unwrap_or_else(|| "unknown".to_string()),
                persistent: persistent.unwrap_or(false),
            });
        }
    }

    Json(ListWorkloadsResponse {
        total: summaries.len(),
        workloads: summaries,
    })
}

// ============================================================================
// Error Handling
// ============================================================================

/// API error types
#[cfg(feature = "daemon")]
#[derive(Debug)]
#[allow(dead_code)] // Some variants used in later phases
enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

#[cfg(feature = "daemon")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            Self::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

#[cfg(feature = "daemon")]
impl From<crate::CliError> for ApiError {
    fn from(err: crate::CliError) -> Self {
        error!("Internal error: {}", err);
        Self::InternalError(err.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "daemon"))]
#[allow(clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn create_test_state() -> ServerState {
        let workload_manager = Arc::new(
            crate::daemon::WorkloadManager::new(2)
                .await
                .expect("create workload manager"),
        );
        ServerState {
            start_time: std::time::Instant::now(),
            workload_manager,
        }
    }

    async fn read_body(response: axum::response::Response) -> bytes::Bytes {
        use http_body_util::BodyExt;
        response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
    }

    #[tokio::test]
    async fn test_router_construct() {
        let state = create_test_state().await;
        let app = create_router(state);
        assert!(std::mem::size_of_val(&app) > 0);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::OK);
        let body = read_body(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(json["status"], "ok");
        assert!(json["uptime_secs"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::OK);
        let body = read_body(response).await;
        let text = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(text.contains("toadstool_daemon_uptime_seconds"));
        assert!(text.contains("toadstool_workloads_total"));
    }

    #[tokio::test]
    async fn test_list_workloads_endpoint() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workloads")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::OK);
        let body = read_body(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert!(json["workloads"].is_array());
        assert_eq!(json["total"], 0);
    }

    #[tokio::test]
    async fn test_submit_workload_success() {
        let state = create_test_state().await;
        let app = create_router(state);

        let body = serde_json::json!({
            "biome_yaml": "version: 1.0",
            "requester": "test-client",
            "environment": {},
            "persistent": false
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workload/submit")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::OK);
        let resp_body = read_body(response).await;
        let json: serde_json::Value = serde_json::from_slice(&resp_body).expect("parse json");
        assert!(json["workload_id"].as_str().is_some());
        assert_eq!(json["status"], "queued");
    }

    #[tokio::test]
    async fn test_get_workload_not_found() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workload/nonexistent-id")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = read_body(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
        assert_eq!(json["error"], "not_found");
    }

    #[tokio::test]
    async fn test_delete_workload_not_found() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/workload/nonexistent-id")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_submit_workload_invalid_json() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workload/submit")
                    .header("Content-Type", "application/json")
                    .body(Body::from("not valid json"))
                    .expect("build request"),
            )
            .await
            .expect("request");

        // Axum may return 422 (Unprocessable Entity) or 400 (Bad Request) for invalid JSON
        assert!(
            response.status() == StatusCode::UNPROCESSABLE_ENTITY
                || response.status() == StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn test_route_not_found() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("request");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

// ============================================================================
