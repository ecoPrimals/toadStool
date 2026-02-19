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
//! - **UniBin Support**: Main server function for UniBin integration
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
//!         .bind_address(format!("0.0.0.0:{}", toadstool_config::ports::server_port()))
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
//!
//! ## UniBin Integration
//!
//! For UniBin architecture, use the `run_server_main()` function:
//!
//! ```ignore
//! use toadstool_server::run_server_main;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     run_server_main().await
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

// Re-export server functions for daemon
#[deprecated(
    since = "2.2.0",
    note = "Use ManualJsonRpcServer instead - no TCP hardcoding"
)]
#[allow(deprecated)]
pub use tarpc_server::{StandaloneExecutor, ToadStoolTarpcServer, WorkloadExecutor};

// EVOLVED: TestExecutor isolated to testing (deep debt principle)
// Backward compatibility alias for test code
#[cfg(test)]
#[deprecated(since = "2.2.0", note = "Use StandaloneExecutor instead")]
pub use tarpc_server::TestExecutor;

// ⚠️ IMPORTANT: Protocol Priority (wateringHole Standard)
// 1. PRIMARY: JSON-RPC 2.0 over Unix sockets (universal, language-agnostic)
// 2. OPTIONAL: tarpc over Unix sockets (binary RPC for performance-critical paths)
// 3. DEPRECATED: HTTP/TCP (use Songbird for HTTP/TLS)
//
// Per PRIMAL_IPC_PROTOCOL.md and UNIVERSAL_IPC_STANDARD_V3.md:
// JSON-RPC 2.0 is the REQUIRED protocol for inter-primal communication.
// tarpc is OPTIONAL for internal high-performance paths.
//
// See manual_jsonrpc::ManualJsonRpcServer::serve() and tarpc_server::serve_unix()
// for correct implementations.

// EVOLVED: Test exports properly isolated
#[cfg(test)]
pub use mocks::*;

// Module declarations
pub mod background;
pub mod capabilities; // Self-knowledge & peer discovery
pub mod config;
pub mod coordinator_executor;
pub mod cross_gate; // Cross-gate compute delegation (job routing across mesh)
pub mod errors;
pub mod gpu_job_queue; // GPU compute job queue (compute.submit/status/result/cancel/list)

// Graph types for collaborative intelligence - modularized for code size compliance
pub mod graph_edge;
pub mod graph_errors;
pub mod graph_node;
pub mod graph_types; // Main graph types (ExecutionGraph, builders)

pub mod handlers;

// ✅ PURE RUST: Manual JSON-RPC 2.0 over Unix sockets (no jsonrpsee!)
pub mod manual_jsonrpc;

// ✅ PURE RUST: Ollama integration client (ollama.list_models/inference/load/unload)
pub mod ollama;

// ✅ EVOLVED: Mocks isolated to testing (deep debt principle)
#[cfg(test)]
pub mod mocks;

// ✅ PURE RUST: BearDog's pattern for JSON-RPC 2.0 (no jsonrpsee!)
pub mod pure_jsonrpc;
pub mod resource_estimator;
pub mod resource_optimizer;
pub mod resource_validator;
pub mod rpc_types; // Pure RPC types (no HTTP deps)
pub mod state;
pub mod tarpc_server;
pub mod unibin; // UniBin server entry point (shared between binaries)

// Re-export background services for tests
pub use background::start_background_services;

// Re-export UniBin entry point for external use
pub use unibin::run_server_main;

// Re-export pure RPC types (deep debt solution)
pub use rpc_types::*;

// Re-export coordinator executor
pub use coordinator_executor::CoordinatorExecutor;

// Re-export manual JSON-RPC server
pub use manual_jsonrpc::ManualJsonRpcServer;

// Re-export collaborative intelligence types
pub use graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, GraphValidationError, NodeResourceRequirements,
};
pub use resource_estimator::{EstimationError, NodeEstimate, ResourceEstimate, ResourceEstimator};
pub use resource_optimizer::{
    Bottleneck, ImprovementEstimate, Opportunity, OptimizationSuggestions, ResourceOptimizer,
};
pub use resource_validator::{
    AvailabilityResult, ResourceGap, ResourceValidator, SystemCapabilities, ValidationError,
};

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
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// during graceful shutdown of background services or active connections.
    #[must_use = "Server shutdown should be checked"]
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
    use std::time::Duration;
    use toadstool_testing::mocks::MockRuntimeEngine;

    // ========================================================================
    // Config struct Default implementations
    // ========================================================================

    #[test]
    fn test_authentication_config_default() {
        let config = AuthenticationConfig::default();
        assert!(!config.required);
        assert!(config.api_keys.is_empty());
        assert!(config.jwt_secret.is_none());
        assert!(config.basic_auth.is_empty());
        assert!(config.custom_validator.is_none());
    }

    #[test]
    fn test_rate_limiting_config_default() {
        let config = RateLimitingConfig::default();
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.concurrent_executions_per_client, 10);
        assert!(config.limit_by_ip);
        assert!(config.limit_by_api_key);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.log_requests);
        assert!(config.log_executions);
        assert!(config.log_metrics);
    }

    #[test]
    fn test_health_check_config_default() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert!(config.check_runtime_engines);
        assert!(config.check_resources);
        assert_eq!(config.memory_threshold_percent, 90.0);
        assert_eq!(config.cpu_threshold_percent, 95.0);
    }

    #[test]
    fn test_primal_capabilities_config_default() {
        let config = PrimalCapabilitiesConfig::default();
        assert!(config.heartbeat_interval_secs > 0);
        assert!(config.heartbeat_interval_secs <= 86400); // Sanity: not more than 24h
    }

    #[test]
    fn test_server_statistics_default() {
        let stats = ServerStatistics::default();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.successful_executions, 0);
        assert_eq!(stats.failed_executions, 0);
        assert!((stats.average_execution_time_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.peak_concurrent_executions, 0);
        assert_eq!(stats.uptime_seconds, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.errors_count, 0);
    }

    // ========================================================================
    // ServerConfig builder pattern
    // ========================================================================

    #[test]
    #[allow(deprecated)] // Testing config with deprecated fields during migration
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
        assert!(!config.enable_websocket); // Disabled by default for security
        assert_eq!(config.max_concurrent_executions, 100);
    }

    #[test]
    fn test_server_config_builder() {
        let server_port = toadstool_config::ports::server_port();
        let config = ServerConfig::default()
            .bind_address(format!("0.0.0.0:{}", server_port))
            .enable_api(false)
            .max_concurrent_executions(50);

        assert_eq!(config.bind_address, format!("0.0.0.0:{}", server_port));
        assert!(!config.enable_api);
        assert_eq!(config.max_concurrent_executions, 50);
    }

    #[test]
    fn test_server_config_builder_enable_websocket() {
        let config = ServerConfig::default()
            .bind_address("127.0.0.1:0".to_string())
            .enable_websocket(true);
        assert!(config.enable_websocket);
    }

    #[test]
    fn test_server_config_builder_default_timeout() {
        let timeout = Duration::from_secs(60);
        let config = ServerConfig::default()
            .bind_address("127.0.0.1:0".to_string())
            .default_timeout(timeout);
        assert_eq!(config.default_timeout, timeout);
    }

    #[test]
    fn test_server_config_builder_auth() {
        let auth = AuthenticationConfig::default();
        let config = ServerConfig::default()
            .bind_address("127.0.0.1:0".to_string())
            .auth(auth);
        assert!(config.auth.is_some());
    }

    #[test]
    fn test_server_config_builder_rate_limiting() {
        let rate_limiting = RateLimitingConfig::default();
        let config = ServerConfig::default()
            .bind_address("127.0.0.1:0".to_string())
            .rate_limiting(rate_limiting);
        assert!(config.rate_limiting.is_some());
    }

    // ========================================================================
    // ToadStoolServer impl
    // ========================================================================

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
        assert!(server.router.is_some());
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

    // ========================================================================
    // ServerError Display (via re-export)
    // ========================================================================

    #[test]
    fn test_server_error_display() {
        let err = ServerError::Configuration("invalid addr".to_string());
        let s = err.to_string();
        assert!(s.contains("Invalid configuration"));
        assert!(s.contains("invalid addr"));
    }

    // ========================================================================
    // ServerEvent variants (Debug format)
    // ========================================================================

    #[test]
    fn test_server_event_execution_started() {
        let event = ServerEvent::ExecutionStarted {
            execution_id: uuid::Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("ExecutionStarted"));
    }

    #[test]
    fn test_server_event_execution_completed() {
        let event = ServerEvent::ExecutionCompleted {
            execution_id: uuid::Uuid::new_v4(),
            status: toadstool::ExecutionStatus::Success,
            duration_ms: 100,
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("ExecutionCompleted"));
    }

    #[test]
    fn test_server_event_runtime_engine_registered() {
        let event = ServerEvent::RuntimeEngineRegistered {
            runtime_type: RuntimeType::Wasm,
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("RuntimeEngineRegistered"));
    }

    #[test]
    fn test_server_event_resource_usage_update() {
        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            active_executions: 2,
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("ResourceUsageUpdate"));
    }

    #[test]
    fn test_server_event_health_status_changed() {
        let event = ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "ok".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("HealthStatusChanged"));
    }

    #[test]
    fn test_server_event_error_occurred() {
        let event = ServerEvent::ErrorOccurred {
            error_type: "Network".to_string(),
            message: "connection failed".to_string(),
            execution_id: None,
            timestamp: chrono::Utc::now(),
        };
        let formatted = format!("{event:?}");
        assert!(formatted.contains("ErrorOccurred"));
    }
}
