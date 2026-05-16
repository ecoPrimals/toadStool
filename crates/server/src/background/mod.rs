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
//! - [`pcie_keepalive`] — PCIe bridge keepalive + hierarchy pinning (prevents D3cold)

mod capability;
mod cleanup;
mod health;
pub(crate) mod pcie_keepalive;
mod resource;
mod statistics;

use tracing::info;

use toadstool::RuntimeEngine;

use crate::state::ServerState;

// Re-export for unit tests (only used in #[cfg(test)] mod tests)
#[cfg(test)]
pub(crate) use health::perform_health_check;

/// Start all background services
pub async fn start_background_services<E: RuntimeEngine + 'static>(state: ServerState<E>) {
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

    // Start PCIe bridge keepalive — pins all GPU bridge hierarchies and
    // generates periodic CfgRd traffic to prevent D3cold (critical for PLX,
    // AMD switches, and any multi-level PCIe topology)
    tokio::spawn(async move {
        pcie_keepalive::run().await;
    });

    info!("Background services started");

    // Background tasks will continue running until they're aborted or process exits
    // No need for an infinite loop here - the spawned tasks run independently
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
