//! HTTP API server for ToadStool daemon mode
//!
//! Implements the REST API for workload submission and management.

#[cfg(feature = "daemon")]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
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
fn create_router(state: ServerState) -> Router {
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
        .map_err(|e| ApiError::InternalError(format!("Failed to submit workload: {}", e)))?;

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
        .ok_or_else(|| ApiError::NotFound(format!("Workload {} not found", id)))?;

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
        .map_err(|e| ApiError::NotFound(format!("Workload {} not found: {}", id, e)))?;

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
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg)
            }
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
        ApiError::InternalError(err.to_string())
    }
}

// ============================================================================
