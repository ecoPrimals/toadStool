// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server startup and shutdown lifecycle
//!
//! Handles HTTP server binding, serving, and graceful shutdown of active executions.

use std::net::SocketAddr;

use axum::Router;
use toadstool::ExecutionStatus;
use tracing::{debug, info};

use crate::errors::{ServerError, ServerResult};
use crate::state::{ServerEvent, ServerState};

/// Bind to the given address and serve the router.
///
/// # Errors
/// Returns `ServerError` if the address cannot be parsed, binding fails, or serving fails.
pub async fn bind_and_serve(addr: SocketAddr, router: Router) -> ServerResult<()> {
    info!("ToadStool server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| ServerError::Network(format!("Failed to bind to address: {e}")))?;

    axum::serve(listener, router.into_make_service())
        .await
        .map_err(|e| ServerError::Internal(format!("Server error: {e}")))?;

    Ok(())
}

/// Cancel all active executions and broadcast completion events.
///
/// Called during graceful shutdown to clean up in-flight workloads.
///
/// # Errors
///
/// This implementation does not fail; returns [`ServerResult`] for API consistency.
pub async fn cancel_all_executions(state: &ServerState) -> ServerResult<()> {
    info!("Cancelling active executions during shutdown");

    let mut active_executions = state.active_executions.write().await;
    for (id, execution) in active_executions.drain() {
        debug!("Cancelling execution {} during shutdown", id);

        let _ = state
            .event_broadcaster
            .send(ServerEvent::ExecutionCompleted {
                execution_id: id,
                status: ExecutionStatus::Cancelled,
                #[allow(clippy::cast_possible_truncation)]
                duration_ms: std::time::SystemTime::now()
                    .duration_since(execution.started_at)
                    .unwrap_or_default()
                    .as_millis() as u64, // fits: duration < u64::MAX ms
                timestamp: std::time::SystemTime::now(),
            });
    }

    info!("Shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    // Test helper uses SystemResourceMonitor (real implementation). Production
    // lifecycle receives ServerState from ToadStoolServer::new() which uses SystemResourceMonitor.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{broadcast, RwLock};
    use uuid::Uuid;

    use crate::state::{ActiveExecution, ClientInfo, ServerState, ServerStatistics};
    use toadstool::ExecutionStatus;
    use toadstool::RuntimeType;
    use toadstool::SystemResourceMonitor;

    fn create_test_state() -> ServerState {
        let (event_broadcaster, _) = broadcast::channel(100);
        ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config: crate::config::ServerConfig::default(),
            resource_monitor: Arc::new(SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_cancel_all_executions_empty() {
        let state = create_test_state();
        let result = cancel_all_executions(&state).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_cancel_all_executions_with_active() {
        let state = create_test_state();
        {
            let mut active = state.active_executions.write().await;
            active.insert(
                Uuid::new_v4(),
                ActiveExecution {
                    execution_id: Uuid::new_v4(),
                    runtime_type: RuntimeType::Native,
                    started_at: std::time::SystemTime::now(),
                    timeout: Duration::from_secs(60),
                    status: ExecutionStatus::Running,
                    client_info: ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
        let result = cancel_all_executions(&state).await;
        assert!(result.is_ok());
        let active = state.active_executions.read().await;
        assert!(active.is_empty());
    }
}
