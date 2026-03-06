// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability heartbeat background task
//!
//! Sends periodic heartbeats to registered primals (Songbird, Squirrel)
//! when capability provider is configured.

use std::time::Duration;

use tracing::{debug, warn};

use crate::state::ServerState;
use toadstool_common::constants::timeouts::HEALTH_CHECK_INTERVAL;
use tokio::time::interval;

/// Capability heartbeat background task
pub(super) async fn run(state: ServerState) {
    debug!("Starting capability heartbeat task");

    let heartbeat_interval = if let Some(ref primal_config) = state.config.primal_capabilities {
        Duration::from_secs(primal_config.heartbeat_interval_secs)
    } else {
        HEALTH_CHECK_INTERVAL // 30 second fallback
    };

    let mut interval = interval(heartbeat_interval);

    loop {
        interval.tick().await;

        if let Some(ref provider) = state.capability_provider {
            debug!("Sending capability heartbeat to all registered primals");

            if let Err(e) = provider.send_heartbeats().await {
                warn!("Failed to send heartbeats: {:?}", e);
            } else {
                debug!("Heartbeats sent successfully to all registered primals");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    use crate::config::ServerConfig;
    use crate::state::{ServerState, ServerStatistics};

    #[tokio::test]
    async fn test_capability_run_no_provider_completes_one_tick() {
        let (event_broadcaster, _) = broadcast::channel(100);
        let state = ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config: ServerConfig::default(),
            resource_monitor: Arc::new(
                toadstool_testing::mocks::resource_monitors::MockResourceMonitor::new_successful(),
            ),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider: None,
        };

        let handle = tokio::spawn(async move {
            run(state).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;
    }
}
