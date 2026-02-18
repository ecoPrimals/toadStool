//! # Toadstool BYOB Server
//!
//! HTTP server for handling BYOB deployment requests from Songbird.
//! Provides compute execution capabilities for team biome deployments.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Json, Path, State},
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use toadstool::{
    byob::{
        create_byob_executor, ByobDeploymentRequest, ByobDeploymentResponse, ByobExecutor,
        ByobExecutorConfig, ResourceUsage,
    },
    RuntimeEngine, ToadStoolError, ToadStoolResult,
};
use toadstool_api::byob::{ApiError, HealthResponse, StopDeploymentResponse};
use toadstool_common::constants::network::{BIND_ALL_IPV4, BYOB_DEFAULT_PORT};
use toadstool_runtime_container::ContainerRuntimeEngine;
// use toadstool_config::constants::network::DEFAULT_TOADSTOOL_PORT;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "toadstool-byob-server")]
#[command(about = "Toadstool BYOB Server - Compute execution for team biomes")]
struct Args {
    /// Server bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Server port
    #[arg(short, long, default_value_t = BYOB_DEFAULT_PORT)]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// BYOB executor configuration
    pub byob_config: ByobExecutorConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: BIND_ALL_IPV4.to_string(),
            port: BYOB_DEFAULT_PORT,
            byob_config: ByobExecutorConfig::default(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let config = load_config(args.config.as_deref()).await?;

    // Create runtime engine
    let runtime_engine = create_runtime_engine().await?;

    // Create BYOB executor
    let byob_executor = create_byob_executor(runtime_engine);

    // Create stateless router first, then add state at the end
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        // Add BYOB routes directly
        .route("/byob/deploy", post(byob_deploy_handler))
        .route("/byob/deployments", get(byob_list_deployments_handler))
        .route(
            "/byob/deployments/:deployment_id",
            get(byob_get_deployment_status_handler),
        )
        .route(
            "/byob/deployments/:deployment_id/stop",
            post(byob_stop_deployment_handler),
        )
        .route(
            "/byob/deployments/:deployment_id/usage",
            get(byob_get_resource_usage_handler),
        )
        .route("/byob/health", get(byob_health_check_handler))
        .with_state(byob_executor);

    // Create server address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting Toadstool BYOB Server on {}", addr);

    // Start server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// Load server configuration
async fn load_config(config_path: Option<&str>) -> ToadStoolResult<ServerConfig> {
    if let Some(path) = config_path {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToadStoolError::configuration(format!("Failed to read config: {}", e)))?;

        let config: ServerConfig = toml::from_str(&content)
            .map_err(|e| ToadStoolError::configuration(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    } else {
        Ok(ServerConfig::default())
    }
}

/// Create runtime engine
async fn create_runtime_engine() -> ToadStoolResult<Arc<dyn RuntimeEngine>> {
    info!("Initializing container runtime engine");

    let engine = ContainerRuntimeEngine::new()?;

    Ok(Arc::new(engine))
}

/// Root handler
async fn root_handler() -> &'static str {
    "🍄 Toadstool BYOB Server - Ready for team biome deployments!"
}

/// Health check handler
async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "toadstool-byob-server",
        "version": env!("CARGO_PKG_VERSION"),
        "message": "Ready to execute team biomes"
    }))
}

// BYOB API handlers
async fn byob_deploy_handler(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Json(request): Json<ByobDeploymentRequest>,
) -> Result<Json<ByobDeploymentResponse>, ApiError> {
    match executor.deploy_biome(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn byob_list_deployments_handler(
    State(executor): State<Arc<dyn ByobExecutor>>,
) -> Result<Json<Vec<ByobDeploymentResponse>>, ApiError> {
    match executor.list_deployments().await {
        Ok(deployments) => Ok(Json(deployments)),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn byob_get_deployment_status_handler(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ByobDeploymentResponse>, ApiError> {
    match executor.get_deployment_status(deployment_id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn byob_stop_deployment_handler(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<StopDeploymentResponse>, ApiError> {
    match executor.stop_deployment(deployment_id).await {
        Ok(()) => Ok(Json(StopDeploymentResponse {
            deployment_id,
            message: "Deployment stopped successfully".to_string(),
        })),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn byob_get_resource_usage_handler(
    State(executor): State<Arc<dyn ByobExecutor>>,
    Path(deployment_id): Path<Uuid>,
) -> Result<Json<ResourceUsage>, ApiError> {
    match executor.get_resource_usage(deployment_id).await {
        Ok(usage) => Ok(Json(usage)),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn byob_health_check_handler() -> Result<Json<HealthResponse>, ApiError> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        message: "Toadstool BYOB API is operational".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_default_config() {
        let config = load_config(None)
            .await
            .expect("Failed to load configuration - check environment variables and config files");
        // Just verify it loads successfully
        assert!(config.port > 0, "Port should be set");
        assert!(
            !config.bind_address.is_empty(),
            "Bind address should be set"
        );
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        // Just verify defaults are set
        assert!(config.port > 0, "Port should be set");
        assert!(
            !config.bind_address.is_empty(),
            "Bind address should be set"
        );
    }
}
