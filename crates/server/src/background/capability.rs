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
