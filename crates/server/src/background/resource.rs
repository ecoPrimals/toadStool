// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource monitoring background task
//!
//! Collects CPU/memory usage, broadcasts ResourceUsageUpdate events,
//! and updates server statistics (uptime, peak concurrent executions).

use tracing::{debug, warn};

use crate::state::{ServerEvent, ServerState};
use tokio::time::interval;

/// Resource monitoring background task
pub(super) async fn run(state: ServerState) {
    debug!("Starting resource monitoring task");

    let mut interval = interval(state.config.resource_monitoring_interval);

    loop {
        interval.tick().await;

        // Get real system resources
        let system_resources = match state.resource_monitor.get_system_resources().await {
            Ok(resources) => resources,
            Err(e) => {
                warn!("Failed to get system resources: {}", e);
                continue;
            }
        };

        // System metrics from resource monitor
        let cpu_usage_percent = system_resources.cpu_usage_percent;
        let memory_usage_percent = system_resources.memory_usage_percent;

        debug!(
            "Resource monitoring: CPU: {:.1}%, Memory: {:.1}%, Available CPU cores: {:.1}, Available Memory: {} bytes",
            cpu_usage_percent,
            memory_usage_percent,
            system_resources.available_cpu_cores,
            system_resources.available_memory_bytes
        );

        let active_executions =
            u32::try_from(state.active_executions.read().await.len()).unwrap_or(0);

        // Deep Debt: Log if broadcast fails (normal when no clients connected)
        if state
            .event_broadcaster
            .send(ServerEvent::ResourceUsageUpdate {
                cpu_usage_percent,
                memory_usage_percent,
                active_executions,
                timestamp: std::time::SystemTime::now(),
            })
            .is_err()
        {
            // Only log at trace level - this happens constantly when no clients
            tracing::trace!("No event receivers for ResourceUsageUpdate");
        }

        // Update statistics
        let mut stats = state.stats.write().await;
        stats.uptime_seconds += state.config.resource_monitoring_interval.as_secs();
        if active_executions > stats.peak_concurrent_executions {
            stats.peak_concurrent_executions = active_executions;
        }
    }
}
