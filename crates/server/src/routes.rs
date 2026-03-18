// SPDX-License-Identifier: AGPL-3.0-or-later
//! HTTP route setup and handler registration
//!
//! Builds the Axum router with health, API, and dashboard endpoints.

use axum::{Router, routing::delete, routing::get, routing::post};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::config::ServerConfig;
use crate::handlers;
use crate::state::ServerState;

/// Build the HTTP router with all routes.
///
/// Registers health endpoints, optional API endpoints (when `enable_api` is true),
/// dashboard, and CORS when enabled.
pub fn build_router(config: &ServerConfig, state: ServerState) -> Router {
    info!("Building server router");

    let mut router = Router::new();

    // Health endpoints
    router = router
        .route("/health", get(handlers::health_check_handler))
        .route("/ready", get(handlers::readiness_check_handler))
        .route("/metrics", get(handlers::metrics_handler));

    // API endpoints
    if config.enable_api {
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
    if config.enable_cors {
        router = router.layer(CorsLayer::permissive());
    }

    // Add state
    router.with_state(state)
}

#[cfg(all(test, feature = "api"))]
mod tests {
    // Test helper uses SystemResourceMonitor (real implementation). Production
    // routes receive ServerState from ToadStoolServer::new() which uses SystemResourceMonitor.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{RwLock, broadcast};

    use crate::state::{ServerState, ServerStatistics};
    use toadstool::SystemResourceMonitor;

    fn create_test_state(config: ServerConfig) -> ServerState {
        let (event_broadcaster, _) = broadcast::channel(100);
        ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config,
            resource_monitor: Arc::new(SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider: None,
        }
    }

    #[test]
    fn test_build_router() {
        let config = ServerConfig::default();
        let state = create_test_state(config.clone());
        let router = build_router(&config, state);
        drop(router);
    }

    #[test]
    fn test_build_router_with_api_disabled() {
        let config = ServerConfig::default().enable_api(false);
        let state = create_test_state(config.clone());
        let router = build_router(&config, state);
        drop(router);
    }
}
