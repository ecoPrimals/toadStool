// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cleanup background task
//!
//! Garbage collection for timed-out executions. Removes stale entries
//! from active_executions and broadcasts ExecutionCompleted (Failed) events.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool::RuntimeEngine;

use crate::state::{ActiveExecution, ServerEvent, ServerState};
use toadstool_common::constants::timeouts::DEFAULT_CACHE_TTL;
use tokio::time::interval;

/// Cleanup background task
pub(super) async fn run<E: RuntimeEngine>(state: ServerState<E>) {
    debug!("Starting cleanup task");

    let mut interval = interval(DEFAULT_CACHE_TTL); // Cleanup every 5 minutes

    loop {
        interval.tick().await;

        let mut active_executions = state.active_executions.write().await;
        let now = SystemTime::now();
        let to_remove = find_timed_out_execution_ids(&active_executions, now);
        let cleanup_count = to_remove.len();
        for id in to_remove {
            if let Some(execution) = active_executions.remove(&id) {
                warn!("Cleaning up timed-out execution: {}", id);

                // Deep Debt: Log if broadcast fails (important - execution completed)
                if state
                    .event_broadcaster
                    .send(ServerEvent::ExecutionCompleted {
                        execution_id: id,
                        status: toadstool::ExecutionStatus::Failed {
                            error: std::borrow::Cow::Borrowed("Execution timed out"),
                        },
                        duration_ms: u64::try_from(execution.timeout.as_millis()).unwrap_or(0),
                        timestamp: now,
                    })
                    .is_err()
                {
                    tracing::debug!("No event receivers for ExecutionCompleted (timeout)");
                }
            }
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} timed-out executions", cleanup_count);
        }
    }
}

/// Return execution IDs whose elapsed time exceeds their configured timeout.
///
/// Exposed for unit testing — not part of the public API.
#[doc(hidden)]
pub(crate) fn find_timed_out_execution_ids(
    active_executions: &HashMap<Uuid, ActiveExecution>,
    now: SystemTime,
) -> Vec<Uuid> {
    active_executions
        .iter()
        .filter_map(|(id, execution)| {
            let elapsed = now
                .duration_since(execution.started_at)
                .unwrap_or(Duration::ZERO);
            if elapsed > execution.timeout {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}
