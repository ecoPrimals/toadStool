//! Statistics collection background task
//!
//! Periodic aggregation and logging of server statistics.
//! Real-time stats are updated by other parts of the system.

use std::time::Duration;

use tracing::debug;

use crate::state::ServerState;
use tokio::time::interval;

/// Statistics collection background task
pub(super) async fn run(state: ServerState) {
    debug!("Starting statistics collection task");

    let mut interval = interval(Duration::from_secs(60)); // Collect stats every minute

    loop {
        interval.tick().await;

        let active_executions = state.active_executions.read().await;
        let runtime_engines = state.runtime_engines.read().await;

        debug!(
            "Statistics: Active executions: {}, Runtime engines: {}",
            active_executions.len(),
            runtime_engines.len()
        );

        // Statistics are updated in real-time by other parts of the system
        // This task can be used for periodic aggregation or cleanup
    }
}
