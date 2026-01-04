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
use tokio::sync::RwLock;
#[cfg(feature = "daemon")]
use tower_http::cors::CorsLayer;
#[cfg(feature = "daemon")]
use tower_http::trace::TraceLayer;
#[cfg(feature = "daemon")]
use tracing::{error, info};

#[cfg(feature = "daemon")]
use super::api_types::*;

/// Shared server state
#[cfg(feature = "daemon")]
#[derive(Clone)]
pub struct ServerState {
    /// Server start time
    pub start_time: Instant,
    
    /// biomeOS client (if connected)
    pub biomeos_client: Option<Arc<toadstool::biomeos_integration::BiomeOSClient>>,
    
    /// Active workloads (workload_id -> status)
    pub workloads: Arc<RwLock<std::collections::HashMap<String, WorkloadStatusResponse>>>,
}

/// Start HTTP API server
#[cfg(feature = "daemon")]
pub async fn start_http_server(
    port: u16,
    biomeos_client: Option<Arc<toadstool::biomeos_integration::BiomeOSClient>>,
) -> anyhow::Result<()> {
    let state = ServerState {
        start_time: Instant::now(),
        biomeos_client,
        workloads: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
    let workloads = state.workloads.read().await;
    let active_workloads = workloads
        .values()
        .filter(|w| w.status == WorkloadStatus::Running || w.status == WorkloadStatus::Queued)
        .count();
    
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs,
        active_workloads,
        biomeos_connected: state.biomeos_client.is_some(),
    })
}

/// Metrics handler (Prometheus-compatible)
#[cfg(feature = "daemon")]
async fn metrics_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let workloads = state.workloads.read().await;
    
    let queued = workloads.values().filter(|w| w.status == WorkloadStatus::Queued).count();
    let running = workloads.values().filter(|w| w.status == WorkloadStatus::Running).count();
    let completed = workloads.values().filter(|w| w.status == WorkloadStatus::Completed).count();
    let failed = workloads.values().filter(|w| w.status == WorkloadStatus::Failed).count();
    
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
        if state.biomeos_client.is_some() { 1 } else { 0 }
    );
    
    (StatusCode::OK, metrics)
}

/// Submit workload handler (Phase 3 will implement actual execution)
#[cfg(feature = "daemon")]
async fn submit_workload_handler(
    State(state): State<ServerState>,
    Json(request): Json<SubmitWorkloadRequest>,
) -> Result<Json<SubmitWorkloadResponse>, ApiError> {
    info!("📥 Received workload submission from: {}", request.requester);
    
    // TODO Phase 3: Implement actual workload execution
    // For now, just queue it and return a workload ID
    
    let workload_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let status_response = WorkloadStatusResponse {
        workload_id: workload_id.clone(),
        status: WorkloadStatus::Queued,
        started_at: Some(now.clone()),
        completed_at: None,
        exit_code: None,
        error: None,
        resource_usage: None,
    };
    
    state.workloads.write().await.insert(workload_id.clone(), status_response);
    
    info!("✅ Workload queued: {}", workload_id);
    
    Ok(Json(SubmitWorkloadResponse {
        workload_id,
        status: WorkloadStatus::Queued,
        message: "Workload queued successfully (Phase 3 will implement execution)".to_string(),
    }))
}

/// Get workload status handler
#[cfg(feature = "daemon")]
async fn get_workload_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<WorkloadStatusResponse>, ApiError> {
    let workloads = state.workloads.read().await;
    
    let status = workloads
        .get(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Workload {} not found", id)))?;
    
    Ok(Json(status.clone()))
}

/// Delete workload handler
#[cfg(feature = "daemon")]
async fn delete_workload_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    info!("🗑️  Deleting workload: {}", id);
    
    let mut workloads = state.workloads.write().await;
    
    if workloads.remove(&id).is_some() {
        info!("✅ Workload deleted: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Workload {} not found", id)))
    }
}

/// List workloads handler
#[cfg(feature = "daemon")]
async fn list_workloads_handler(
    State(state): State<ServerState>,
) -> impl IntoResponse {
    let workloads = state.workloads.read().await;
    
    let summaries: Vec<WorkloadSummary> = workloads
        .values()
        .map(|w| WorkloadSummary {
            workload_id: w.workload_id.clone(),
            status: w.status,
            requester: "unknown".to_string(), // TODO: Store requester in workload
            started_at: w.started_at.clone().unwrap_or_else(|| "unknown".to_string()),
            persistent: false, // TODO: Store persistent flag
        })
        .collect();
    
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
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                msg,
            ),
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
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        error!("Internal error: {}", err);
        ApiError::InternalError(err.to_string())
    }
}

// ============================================================================
// Non-daemon stub (when feature is disabled)
// ============================================================================

#[cfg(not(feature = "daemon"))]
pub async fn start_http_server(
    _port: u16,
    _biomeos_client: Option<std::sync::Arc<()>>,
) -> anyhow::Result<()> {
    anyhow::bail!("Daemon mode requires the 'daemon' feature to be enabled")
}

