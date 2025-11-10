//! `ToadStool` Modern API & Interface Layer
//!
//! This module provides a modern, type-safe API interface with:
//! - `OpenAPI` 3.0 compliant REST endpoints
//! - Type-safe request/response structures
//! - Modern async-first patterns
//! - Comprehensive error handling
//! - Real-time WebSocket with structured events
//! - Authentication and authorization
//! - Service mesh integration

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{State, WebSocketUpgrade},
    middleware::{from_fn, from_fn_with_state},
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use chrono::Utc;
use clap::{Parser, Subcommand};

use tokio::sync::{broadcast, RwLock};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use toadstool::{ToadStoolError, ToadStoolResult};

use crate::types::{
    AlertSeverity, ApiConfig, ApiEvent, ApiMetrics, AuthClaims, AuthRequest, AuthResponse,
    ClusterCapacity, ClusterNodeInfo, ClusterStatusResponse, ExecutionFilter, ExecutionInfo,
    ExecutionLogs, ExecutionMetrics, ExecutionRequest, ExecutionResponse, ExecutionStatus,
    HealthCheck, HealthResponse, LogEntry, LogLevel, MetricPoint, MonitoringEndpoints,
    NodeResources, NodeStatus, PaginationInfo, ResourceAllocation, ResourceRequirements,
    ResourceUsage, TimeRange, WorkloadSpec,
};

// Import local modules
pub mod byob;
pub mod handlers;
pub mod middleware;
pub mod types;
pub mod websocket;

// Re-export commonly used types
pub use byob::ByobApi;
pub use types::{ApiError, PaginatedResponse};

/// `OpenAPI` documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::submit_execution,
        handlers::get_execution_status,
        handlers::list_executions,
        handlers::cancel_execution,
        handlers::get_execution_logs,
        handlers::get_execution_metrics,
        handlers::get_cluster_status,
        handlers::health_check,
        handlers::get_api_metrics
    ),
    components(
        schemas(
            ExecutionRequest,
            ExecutionResponse,
            ExecutionStatus,
            ExecutionInfo,
            ClusterStatusResponse,
            ApiError,
            ApiEvent,
            PaginatedResponse<ExecutionInfo>,
            ExecutionFilter,
            WorkloadSpec,
            ResourceRequirements,
            ResourceAllocation,
            MonitoringEndpoints,
            ResourceUsage,
            ClusterCapacity,
            ClusterNodeInfo,
            NodeStatus,
            NodeResources,
            AlertSeverity,
            ApiMetrics,
            ExecutionLogs,
            LogEntry,
            LogLevel,
            ExecutionMetrics,
            MetricPoint,
            TimeRange,
            PaginationInfo,
            ApiConfig,
            AuthClaims,
            AuthRequest,
            AuthResponse,
            HealthResponse,
            HealthCheck
        )
    ),
    tags(
        (name = "executions", description = "Execution management endpoints"),
        (name = "cluster", description = "Cluster monitoring endpoints"),
        (name = "health", description = "Health check endpoints"),
        (name = "metrics", description = "Metrics and monitoring endpoints")
    ),
    info(
        title = "ToadStool Universal Compute API",
        version = "2.0.0",
        description = "Modern API for ToadStool Universal Compute Platform with OpenAPI 3.0 support",
        contact(
            name = "ToadStool Team",
            url = "https://github.com/toadstool/toadstool",
            email = "team@toadstool.dev"
        ),
        license(
            name = "MIT OR Apache-2.0",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8084", description = "Local development server (use TOADSTOOL_API_PORT to customize)"),
        (url = "https://api.toadstool.dev", description = "Production server")
    )
)]
pub struct ApiDoc;

/// Modern API server state with enhanced capabilities
#[derive(Clone)]
pub struct ApiState {
    pub event_broadcaster: broadcast::Sender<ApiEvent>,
    pub executions: Arc<RwLock<HashMap<Uuid, ExecutionInfo>>>,
    pub config: ApiConfig,
    pub metrics: Arc<RwLock<ApiMetrics>>,
    pub websocket_manager: Arc<websocket::WebSocketManager>,
}

/// Modern WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket::handle_websocket(socket, state))
}

/// Modern dashboard with enhanced features
pub async fn serve_dashboard() -> Html<&'static str> {
    Html(include_str!("../templates/dashboard.html"))
}

/// Modern API server with enhanced capabilities
pub struct ModernApiServer {
    config: ApiConfig,
    state: ApiState,
}

impl ModernApiServer {
    /// Create a new modern API server
    #[must_use]
    pub fn new(config: ApiConfig) -> Self {
        let (event_sender, _) = broadcast::channel(10000);

        let state = ApiState {
            event_broadcaster: event_sender,
            executions: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            websocket_manager: Arc::new(websocket::WebSocketManager::new()),
        };

        Self { config, state }
    }

    /// Start the modern API server with full middleware stack
    pub async fn start(&self) -> ToadStoolResult<()> {
        info!(
            "Starting modern API server v{} on {}",
            self.config.api_version, self.config.bind_address
        );

        let mut app = self.build_router()?;

        // Add comprehensive middleware stack
        app = app.layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(
                    self.config.request_timeout_secs,
                )))
                .layer(from_fn_with_state(
                    self.state.clone(),
                    middleware::metrics_middleware,
                ))
                .layer(from_fn(middleware::request_id_middleware))
                .layer(CorsLayer::permissive())
                .into_inner(),
        );

        // Parse address
        let _addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| ToadStoolError::configuration(format!("Invalid bind address: {e}")))?;

        // Start the server
        let listener = tokio::net::TcpListener::bind(&self.config.bind_address).await?;

        info!(
            "🚀 ToadStool API server starting on {}",
            self.config.bind_address
        );

        axum::serve(listener, app.into_make_service())
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Server error: {e}")))?;

        Ok(())
    }

    /// Build the modern router with all endpoints
    fn build_router(&self) -> ToadStoolResult<Router<()>> {
        let mut app = Router::new();

        // API v2 routes with modern handlers
        if self.config.enable_rest {
            app = app
                .route("/api/v2/executions", post(handlers::submit_execution))
                .route("/api/v2/executions", get(handlers::list_executions))
                .route(
                    "/api/v2/executions/:execution_id",
                    get(handlers::get_execution_status),
                )
                .route(
                    "/api/v2/executions/:execution_id",
                    delete(handlers::cancel_execution),
                )
                .route(
                    "/api/v2/executions/:execution_id/logs",
                    get(handlers::get_execution_logs),
                )
                .route(
                    "/api/v2/executions/:execution_id/metrics",
                    get(handlers::get_execution_metrics),
                )
                .route("/api/v2/cluster/status", get(handlers::get_cluster_status))
                .route("/api/v2/health", get(handlers::health_check))
                .route("/api/v2/metrics", get(handlers::get_api_metrics));
        }

        // WebSocket endpoints
        if self.config.enable_websocket {
            app = app.route("/ws", get(websocket_handler));
        }

        // OpenAPI documentation
        if self.config.enable_openapi {
            app = app
                .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
                .route(
                    "/api-docs/openapi.json",
                    get(|| async { Json(ApiDoc::openapi()) }),
                );
        }

        // Dashboard and static routes
        app = app
            .route("/", get(serve_dashboard))
            .route("/dashboard", get(serve_dashboard));

        // BYOB API integration - temporarily disabled due to missing mock
        // let byob_api = byob::ByobApi::new(Arc::new(byob::MockByobExecutor::new()));
        // app = app.merge(byob_api.router());

        // Add state
        let app = app.with_state(self.state.clone());

        Ok(app)
    }

    /// Start background tasks for monitoring and maintenance
    fn _start_background_tasks(&self) {
        let state = self.state.clone();

        // Metrics collection task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;

                // Update uptime and other metrics
                let mut metrics = state.metrics.write().await;
                metrics.uptime_seconds += 60;

                // Broadcast metrics update
                let event = ApiEvent::MetricsUpdated {
                    metrics: metrics.clone(),
                    timestamp: Utc::now(),
                };
                let _ = state.event_broadcaster.send(event);
            }
        });

        // Cleanup task for old executions
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;

                let mut executions = state.executions.write().await;
                let cutoff = Utc::now() - chrono::Duration::hours(24);

                // Remove executions older than 24 hours
                executions.retain(|_, exec| exec.submitted_at > cutoff);

                info!("Cleaned up old executions, {} remaining", executions.len());
            }
        });
    }

    /// Broadcast an event to all connected clients
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
        #[arg(short, long, value_parser)]
        bind: Option<String>,
    },
    /// Submit an execution request
    Execute {
        #[arg(short, long)]
        workload: String,
        #[arg(short, long, default_value = "native")]
        runtime: String,
    },
    /// Get execution status
    Status { execution_id: String },
    /// Monitor cluster health
    Monitor,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::RuntimeType;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        // The default bind address is now environment-aware
        let env_config = toadstool_config::env_config::EnvironmentConfig::from_env();
        assert_eq!(
            config.bind_address,
            format!(
                "{}:{}",
                env_config.network.bind_address, env_config.network.songbird_port
            )
        );
        assert!(config.enable_rest);
        assert!(config.enable_websocket);
        assert!(config.cors_enabled);
    }

    #[test]
    fn test_execution_info_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let execution_id = Uuid::new_v4();
        let runtime_type = RuntimeType::Native;
        let info = types::ExecutionInfo {
            execution_id,
            status: ExecutionStatus::Running,
            runtime_type,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(50.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&info)
            .map_err(|e| ToadStoolError::validation(format!("Failed to serialize info: {e}")))?;
        assert!(json.contains("execution_id"));
        assert!(json.contains("running"));
        Ok(())
    }
}
