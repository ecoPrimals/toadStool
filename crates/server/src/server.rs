// SPDX-License-Identifier: AGPL-3.0-only
//! Main ToadStool server implementation
//!
//! Orchestrates runtime engines, HTTP API, and lifecycle management.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use toadstool::{RuntimeEngine, RuntimeType};

use crate::config::ServerConfig;
use crate::errors::{ServerError, ServerResult};
use crate::lifecycle;
use crate::routes;
use crate::state::{ServerEvent, ServerState, ServerStatistics};

/// Main `ToadStool` server implementation
pub struct ToadStoolServer {
    config: ServerConfig,
    state: ServerState,
    router: Option<Router>,
}

impl ToadStoolServer {
    /// Create a new `ToadStool` server
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// during initialization of background services or state setup.
    #[must_use = "ToadStoolServer creation should be checked"]
    pub async fn new(config: ServerConfig) -> ServerResult<Self> {
        info!("Initializing ToadStool server with config: {:?}", config);

        let (event_broadcaster, _) = broadcast::channel(1000);

        // Initialize capability provider if configured
        let capability_provider = if let Some(primal_config) = &config.primal_capabilities {
            if primal_config.enabled {
                info!("Initializing primal capability provider");

                use toadstool_distributed::primal_capabilities::CapabilityProvider;
                let provider = Arc::new(CapabilityProvider::default());

                // Auto-register with configured primals
                if primal_config.auto_register {
                    if let Some(ref endpoint) = primal_config.songbird_endpoint {
                        info!("Registering with Songbird at {}", endpoint);
                        if let Err(e) = provider.register_with_primal(endpoint).await {
                            tracing::warn!("Failed to register with Songbird: {:?}", e);
                        } else {
                            info!("Successfully registered with Songbird");
                        }
                    }

                    if let Some(ref endpoint) = primal_config.squirrel_endpoint {
                        info!("Registering with Squirrel at {}", endpoint);
                        if let Err(e) = provider.register_with_primal(endpoint).await {
                            tracing::warn!("Failed to register with Squirrel: {:?}", e);
                        } else {
                            info!("Successfully registered with Squirrel");
                        }
                    }
                }

                Some(provider)
            } else {
                info!("Primal capability provider disabled");
                None
            }
        } else {
            info!("Primal capability provider not configured");
            None
        };

        // Production uses real SystemResourceMonitor from toadstool::resources.
        // Tests use MockResourceMonitor (toadstool_testing) for predictable behavior.
        // Evolution: No mock in production path; SystemResourceMonitor reads /proc (Linux) or equivalent.
        let state = ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config: config.clone(),
            resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider,
        };

        Ok(Self {
            config,
            state,
            router: None,
        })
    }

    /// Register a runtime engine with the server
    ///
    /// # Errors
    /// Returns a `ServerError::Configuration` if the runtime type is unknown or invalid.
    #[must_use = "Runtime engine registration should be checked"]
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
                timestamp: std::time::SystemTime::now(),
            });

        Ok(())
    }

    /// Build the router with all routes
    pub fn build_router(&mut self) -> Router {
        let router = routes::build_router(&self.config, self.state.clone());
        self.router = Some(router.clone());
        router
    }

    /// Start the server
    ///
    /// # Errors
    /// Returns a `ServerError` if:
    /// - The bind address is invalid or cannot be parsed.
    /// - The TCP listener cannot be bound to the specified address.
    /// - The server fails to start accepting connections.
    #[must_use = "Server start should be checked"]
    pub async fn start(&mut self) -> ServerResult<()> {
        info!("Starting ToadStool server on {}", self.config.bind_address);

        // Build router if not already built
        let router = if let Some(router) = &self.router {
            router.clone()
        } else {
            self.build_router()
        };

        // Start background tasks
        crate::background::start_background_services(self.state.clone()).await;

        // Parse bind address
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| ServerError::Configuration(format!("Invalid bind address: {e}")))?;

        // Bind and serve
        lifecycle::bind_and_serve(addr, router).await?;

        Ok(())
    }

    /// Get server statistics
    pub async fn get_statistics(&self) -> ServerStatistics {
        self.state.stats.read().await.clone()
    }

    /// Shutdown the server gracefully
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// during graceful shutdown of background services or active connections.
    #[must_use = "Server shutdown should be checked"]
    pub async fn shutdown(&self) -> ServerResult<()> {
        info!("Shutting down ToadStool server");
        lifecycle::cancel_all_executions(&self.state).await
    }
}

#[cfg(test)]
mod tests {
    // Production uses real RuntimeEngine implementations (native, wasm, etc.).
    // MockRuntimeEngine is test-only for unit tests; never in production paths.
    use super::*;
    use toadstool_testing::mocks::MockRuntimeEngine;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_server_creation() {
        let config = ServerConfig::default();
        let server = ToadStoolServer::new(config).await;
        assert!(server.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_runtime_engine_valid() {
        let config = ServerConfig::default();
        let mut server = ToadStoolServer::new(config).await.unwrap();
        let result = server
            .register_runtime_engine("native", Box::new(MockRuntimeEngine::new()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_runtime_engine_unknown_type() {
        let config = ServerConfig::default();
        let mut server = ToadStoolServer::new(config).await.unwrap();
        let result = server
            .register_runtime_engine("unknown_runtime", Box::new(MockRuntimeEngine::new()))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown runtime type"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_build_router() {
        let config = ServerConfig::default();
        let mut server = ToadStoolServer::new(config).await.unwrap();
        let router = server.build_router();
        // Router should be built without panic
        drop(router);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_statistics() {
        let config = ServerConfig::default();
        let server = ToadStoolServer::new(config).await.unwrap();
        let stats = server.get_statistics().await;
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.successful_executions, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_shutdown() {
        let config = ServerConfig::default();
        let server = ToadStoolServer::new(config).await.unwrap();
        let result = server.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_start_invalid_bind_address() {
        let config = ServerConfig::default().bind_address("not-a-valid-address:12345".to_string());
        let mut server = ToadStoolServer::new(config).await.unwrap();
        let result = server.start().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid bind address"));
    }
}
