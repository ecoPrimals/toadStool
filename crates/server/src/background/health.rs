//! Health monitoring background task
//!
//! Runs periodic health checks (resources, runtime engines, execution count)
//! and broadcasts HealthStatusChanged when status transitions.

use tracing::{debug, info, warn};

use crate::state::{ServerEvent, ServerState};
use tokio::time::interval;

/// Health monitoring background task
pub(super) async fn run(state: ServerState) {
    debug!("Starting health monitoring task");

    let mut interval = interval(state.config.health_check.interval);
    let mut last_healthy = true;

    loop {
        interval.tick().await;

        let healthy = perform_health_check(&state).await;

        if healthy != last_healthy {
            info!(
                "Health status changed: {}",
                if healthy { "healthy" } else { "unhealthy" }
            );

            // Deep Debt: Log if broadcast fails
            if state
                .event_broadcaster
                .send(ServerEvent::HealthStatusChanged {
                    healthy,
                    message: if healthy {
                        "System is healthy"
                    } else {
                        "System is unhealthy"
                    }
                    .to_string(),
                    timestamp: std::time::SystemTime::now(),
                })
                .is_err()
            {
                tracing::debug!("No event receivers for HealthStatusChanged");
            }

            last_healthy = healthy;
        }
    }
}

/// Perform comprehensive health check
///
/// Exposed as pub(crate) for unit testing - not part of public API
#[doc(hidden)]
pub(crate) async fn perform_health_check(state: &ServerState) -> bool {
    let config = &state.config.health_check;

    // Check system resources using real data
    if config.check_resources {
        if let Ok(system_resources) = state.resource_monitor.get_system_resources().await {
            // System metrics from resource monitor
            let cpu_usage_percent = system_resources.cpu_usage_percent;
            let memory_usage_percent = system_resources.memory_usage_percent;

            if cpu_usage_percent > config.cpu_threshold_percent
                || memory_usage_percent > config.memory_threshold_percent
            {
                return false;
            }
        } else {
            // If we can't get system resources, consider it unhealthy
            return false;
        }
    }

    // Check runtime engines
    if config.check_runtime_engines {
        let runtime_engines = state.runtime_engines.read().await;
        if runtime_engines.is_empty() {
            return false;
        }

        // Verify each engine is responding
        for (name, engine) in runtime_engines.iter() {
            // Basic engine health check - verify it can get metrics
            match engine.get_metrics().await {
                Ok(_) => {
                    // Engine is responding
                    continue;
                }
                Err(e) => {
                    warn!("Runtime engine '{:?}' health check failed: {}", name, e);
                    return false;
                }
            }
        }

        // Check if we have at least one working runtime engine
        if runtime_engines.is_empty() {
            warn!("No runtime engines available");
            return false;
        }
    }

    // Check for too many active executions
    let active_executions = state.active_executions.read().await.len();
    if active_executions > state.config.max_concurrent_executions as usize {
        return false;
    }

    true
}
