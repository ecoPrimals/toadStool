//! # Toadstool BYOB Server
//!
//! HTTP server for handling BYOB deployment requests from Songbird.
//! Provides compute execution capabilities for team biome deployments.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::get,
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use toadstool::{
    ByobExecutorConfig, RuntimeEngine, ToadStoolError, ToadStoolResult,
    create_byob_executor,
};
use toadstool_api::ByobApi;
use toadstool_runtime_container::ContainerRuntimeEngine;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "toadstool-byob-server")]
#[command(about = "Toadstool BYOB Server - Compute execution for team biomes")]
struct Args {
    /// Server bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Server port
    #[arg(short, long, default_value = "8081")]
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
            bind_address: "0.0.0.0".to_string(),
            port: 8081,
            byob_config: ByobExecutorConfig::default(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.verbose { Level::DEBUG } else { Level::INFO })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let config = load_config(args.config.as_deref()).await?;

    // Create runtime engine
    let runtime_engine = create_runtime_engine().await?;

    // Create BYOB executor
    let byob_executor = create_byob_executor(runtime_engine);

    // Create API router
    let byob_api = ByobApi::new(byob_executor);
    let byob_router = byob_api.router();

    // Create main router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .merge(byob_router);

    // Create server address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting Toadstool BYOB Server on {}", addr);

    // Start server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Load server configuration
async fn load_config(config_path: Option<&str>) -> ToadStoolResult<ServerConfig> {
    if let Some(path) = config_path {
        let content = tokio::fs::read_to_string(path).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_default_config() {
        let config = load_config(None).await.unwrap();
        assert_eq!(config.port, 8081);
        assert_eq!(config.bind_address, "0.0.0.0");
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 8081);
        assert_eq!(config.bind_address, "0.0.0.0");
    }
} 