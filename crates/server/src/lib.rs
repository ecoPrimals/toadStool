//! # `ToadStool` Server Library
//!
//! A comprehensive server library for building `ToadStool` universal compute servers
//! that can accept and execute workloads across multiple runtime engines.
//!
//! ## Features
//!
//! - **HTTP API Server**: REST API for workload submission and status monitoring
//! - **WebSocket Server**: Real-time event streaming and notifications  
//! - **Runtime Engine Integration**: Support for Native, WASM, Container, Python, GPU runtimes
//! - **Load Balancing**: Intelligent workload distribution across available resources
//! - **Resource Management**: CPU, memory, storage, and GPU resource tracking
//! - **Authentication & Authorization**: Configurable security policies
//! - **Ecosystem Integration**: Integration with Songbird, `BearDog`, `NestGate`
//!
//! ## Quick Start
//!
//! ```ignore
//! use toadstool_server::{ToadStoolServer, ServerConfig};
//! use toadstool_runtime_native::NativeRuntimeEngine;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create server configuration
//!     let config = ServerConfig::default()
//!         .bind_address("0.0.0.0:8080")
//!         .enable_api(true)
//!         .enable_websocket(true);
//!     
//!     // Create server instance
//!     let mut server = ToadStoolServer::new(config).await?;
//!     
//!     // Register runtime engines
//!     server.register_runtime_engine("native", Box::new(NativeRuntimeEngine::new())).await?;
//!     
//!     // Start the server
//!     server.start().await?;
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tracing::{debug, info};

use toadstool::{RuntimeEngine, RuntimeType};

// Re-export public types
pub use config::*;
pub use errors::*;
pub use state::*;

#[cfg(test)]
pub use mocks::*;

// Module declarations
pub mod background;
pub mod config;
pub mod errors;
pub mod handlers;
#[cfg(test)]
pub mod mocks;
pub mod state;
pub mod websocket;

/// Main `ToadStool` server implementation
pub struct ToadStoolServer {
    config: ServerConfig,
    state: ServerState,
    router: Option<Router>,
}

impl ToadStoolServer {
    /// Create a new `ToadStool` server
    pub async fn new(config: ServerConfig) -> ServerResult<Self> {
        info!("Initializing ToadStool server with config: {:?}", config);

        let (event_broadcaster, _) = broadcast::channel(1000);

        let state = ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config: config.clone(),
            resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
        };

        Ok(Self {
            config,
            state,
            router: None,
        })
    }

    /// Register a runtime engine with the server
    pub async fn register_runtime_engine(
        &mut self,
        runtime_type: &str,
        engine: Box<dyn RuntimeEngine>,
    ) -> ServerResult<()> {
        let rt_type = match runtime_type {
            "native" => RuntimeType::Native,
            "wasm" => RuntimeType::Wasm,
            "container" => RuntimeType::Container,
            "python" => RuntimeType::Python,
            "gpu" => RuntimeType::Gpu,
            _ => {
                return Err(ServerError::Configuration(format!(
                    "Unknown runtime type: {runtime_type}"
                )))
            }
        };

        info!("Registering runtime engine: {:?}", rt_type);

        let mut engines = self.state.runtime_engines.write().await;
        engines.insert(rt_type.clone(), engine);

        let _ = self
            .state
            .event_broadcaster
            .send(ServerEvent::RuntimeEngineRegistered {
                runtime_type: rt_type,
                timestamp: chrono::Utc::now(),
            });

        Ok(())
    }

    /// Build the router with all routes
    pub fn build_router(&mut self) -> Router {
        info!("Building server router");

        let mut router = Router::new();

        // Health endpoints
        router = router
            .route("/health", get(handlers::health_check_handler))
            .route("/ready", get(handlers::readiness_check_handler))
            .route("/metrics", get(handlers::metrics_handler));

        // API endpoints
        if self.config.enable_api {
            router = router
                .route("/api/executions", post(handlers::submit_execution_handler))
                .route(
                    "/api/executions/:id",
                    get(handlers::get_execution_status_handler),
                )
                .route(
                    "/api/executions/:id",
                    delete(handlers::cancel_execution_handler),
                )
                .route(
                    "/api/cluster/status",
                    get(handlers::get_cluster_status_handler),
                )
                .route(
                    "/api/runtime-engines",
                    get(handlers::list_runtime_engines_handler),
                );
        }

        // WebSocket endpoint
        if self.config.enable_websocket {
            router = router.route("/ws", get(websocket::websocket_handler));
        }

        // Dashboard
        router = router.route("/", get(handlers::dashboard_handler));

        // Add CORS if enabled
        if self.config.enable_cors {
            router = router.layer(CorsLayer::permissive());
        }

        // Add state - this converts Router<ServerState> to Router<()>
        let router = router.with_state(self.state.clone());

        self.router = Some(router.clone());
        router
    }

    /// Start the server
    pub async fn start(&mut self) -> ServerResult<()> {
        info!("Starting ToadStool server on {}", self.config.bind_address);

        // Build router if not already built
        let router = if let Some(router) = &self.router {
            router.clone()
        } else {
            self.build_router()
        };

        // Start background tasks
        background::start_background_services(self.state.clone()).await;

        // Parse bind address
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| ServerError::Configuration(format!("Invalid bind address: {e}")))?;

        // Start the server
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| ServerError::Network(format!("Failed to bind to address: {e}")))?;

        info!("ToadStool server listening on {}", addr);

        // The router already has state from build_router, so we can serve it directly
        axum::serve(listener, router.into_make_service())
            .await
            .map_err(|e| ServerError::Internal(format!("Server error: {e}")))?;

        Ok(())
    }

    /// Get server statistics
    pub async fn get_statistics(&self) -> ServerStatistics {
        self.state.stats.read().await.clone()
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&self) -> ServerResult<()> {
        info!("Shutting down ToadStool server");

        // Cancel all active executions
        let mut active_executions = self.state.active_executions.write().await;
        for (id, execution) in active_executions.drain() {
            debug!("Cancelling execution {} during shutdown", id);

            let _ = self
                .state
                .event_broadcaster
                .send(ServerEvent::ExecutionCompleted {
                    execution_id: id,
                    status: toadstool::ExecutionStatus::Cancelled,
                    duration_ms: chrono::Utc::now()
                        .signed_duration_since(execution.started_at)
                        .num_milliseconds() as u64,
                    timestamp: chrono::Utc::now(),
                });
        }

        info!("ToadStool server shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        // The default bind address is now environment-aware
        let env_config = toadstool_config::env_config::EnvironmentConfig::from_env();
        assert_eq!(
            config.bind_address,
            format!(
                "{}:{}",
                env_config.network.bind_address, env_config.network.songbird_port
            )
        );
        assert!(config.enable_api);
        assert!(config.enable_websocket);
        assert_eq!(config.max_concurrent_executions, 100);
    }

    #[test]
    fn test_server_config_builder() {
        let config = ServerConfig::default()
            .bind_address("0.0.0.0:3000")
            .enable_api(false)
            .max_concurrent_executions(50);

        assert_eq!(config.bind_address, "0.0.0.0:3000");
        assert!(!config.enable_api);
        assert_eq!(config.max_concurrent_executions, 50);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig::default();
        let server = ToadStoolServer::new(config).await;
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_event_serialization() {
        let event = ServerEvent::ExecutionStarted {
            execution_id: uuid::Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            timestamp: chrono::Utc::now(),
        };

        // Test that event can be formatted
        let formatted = format!("{event:?}");
        assert!(formatted.contains("ExecutionStarted"));
    }
}
