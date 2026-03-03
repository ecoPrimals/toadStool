// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! Modern API for ToadStool with OpenAPI support
//!
//! This crate provides both JSON-RPC 2.0 (primary) and REST (legacy) APIs
//! for interacting with the ToadStool universal compute platform.
//!
//! ## ecoBin Compliance: JSON-RPC First
//!
//! Per ecoBin standards, JSON-RPC 2.0 is the **primary** API protocol.
//! REST endpoints are provided for backward compatibility but are **deprecated**.
//!
//! ### Primary: JSON-RPC 2.0
//!
//! ```text
//! POST /jsonrpc
//! Content-Type: application/json
//!
//! {"jsonrpc": "2.0", "method": "api.health", "id": 1}
//! ```
//!
//! ### Legacy: REST (deprecated)
//!
//! REST routes at `/api/v2/*` have been removed. Use JSON-RPC exclusively.

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// Public re-exports. types has 30+ items (ExecutionStatus, WorkloadSpec, ApiError, etc.);
// explicit re-export would be unwieldy.
pub use types::*;

// Internal modules
pub mod byob;
pub mod constants;
pub mod handlers;
pub mod jsonrpc;
pub mod types;

/// API server state
#[derive(Clone)]
pub struct ApiState {
    pub executions: Arc<RwLock<HashMap<Uuid, ExecutionInfo>>>,
    pub metrics: Arc<RwLock<ApiMetrics>>,
    pub event_broadcaster: broadcast::Sender<ApiEvent>,
    /// Capability provider for primal integration (optional for backwards compatibility)
    pub capability_provider:
        Option<Arc<toadstool_distributed::primal_capabilities::CapabilityProvider>>,
}

/// API metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
}

impl Default for ApiMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
        }
    }
}

/// Create the API router with all routes
///
/// ## Route Priority
///
/// 1. **JSON-RPC 2.0** (`/jsonrpc`) - Primary, ecoBin-compliant
/// 2. **REST** (`/api/v2/*`) - Legacy, deprecated
///
/// New integrations should use JSON-RPC exclusively.
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // PRIMARY: JSON-RPC 2.0 (ecoBin compliant)
        .route("/jsonrpc", post(jsonrpc::jsonrpc_handler))
        .route("/", get(root_handler))
        .with_state(state)
}

/// Root handler
async fn root_handler() -> (StatusCode, &'static str) {
    (
        StatusCode::OK,
        "ToadStool API v2 - Universal Compute Platform",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_metrics_default() {
        let metrics = ApiMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.average_response_time_ms, 0.0);
    }

    #[test]
    fn test_api_state_creation() {
        let executions = Arc::new(RwLock::new(HashMap::new()));
        let metrics = Arc::new(RwLock::new(ApiMetrics::default()));
        let (tx, _) = broadcast::channel(100);

        let state = ApiState {
            executions,
            metrics,
            event_broadcaster: tx,
            capability_provider: None,
        };

        assert_eq!(state.executions.try_read().unwrap().len(), 0);
    }
}
