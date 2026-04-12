// SPDX-License-Identifier: AGPL-3.0-or-later
//! Background services for server monitoring and maintenance
//!
//! ## Module structure
//!
#![expect(
    rustdoc::private_intra_doc_links,
    reason = "module-level rustdoc links to private items for maintainer navigation"
)]
//!
//! - [`resource`] — CPU/memory monitoring, ResourceUsageUpdate events
//! - [`health`] — Health checks, HealthStatusChanged events
//! - [`statistics`] — Periodic stats aggregation
//! - [`cleanup`] — Timed-out execution garbage collection
//! - [`capability`] — Primal heartbeat (when capability provider enabled)

mod capability;
mod cleanup;
mod health;
mod resource;
mod statistics;

use tracing::info;

use crate::state::ServerState;

// Re-export for unit tests (only used in #[cfg(test)] mod tests)
#[allow(unused_imports, reason = "re-export for unit tests")]
pub(crate) use health::perform_health_check;

/// Start all background services
pub async fn start_background_services(state: ServerState) {
    info!("Starting background services");

    // Start resource monitoring
    let resource_state = state.clone();
    tokio::spawn(async move {
        resource::run(resource_state).await;
    });

    // Start health monitoring
    let health_state = state.clone();
    tokio::spawn(async move {
        health::run(health_state).await;
    });

    // Start statistics collection
    let stats_state = state.clone();
    tokio::spawn(async move {
        statistics::run(stats_state).await;
    });

    // Start capability heartbeat if enabled
    if state.capability_provider.is_some() {
        let capability_state = state.clone();
        tokio::spawn(async move {
            capability::run(capability_state).await;
        });
    }

    // Start cleanup task
    tokio::spawn(async move {
        cleanup::run(state).await;
    });

    info!("Background services started");

    // Background tasks will continue running until they're aborted or process exits
    // No need for an infinite loop here - the spawned tasks run independently
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
