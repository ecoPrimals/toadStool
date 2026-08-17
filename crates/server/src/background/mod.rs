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
//! - [`catalyst_watchdog`] — Exp 229 lockup sentinel: monitors handoff liveness, emergency quench + kill
//! - [`kernel_sentinel`] — Exp 232 kernel oops sentinel: monitors /dev/kmsg for crash signatures, saves triage reports

#[cfg(feature = "background-monitors")]
mod capability;
#[cfg(target_os = "linux")]
pub(crate) mod catalyst_watchdog;
#[cfg(all(test, target_os = "linux"))]
mod catalyst_watchdog_tests;
#[cfg(feature = "background-monitors")]
mod cleanup;
#[cfg(feature = "background-monitors")]
mod health;
#[cfg(target_os = "linux")]
pub(crate) mod ipc_watch;
#[cfg(target_os = "linux")]
pub(crate) mod kernel_sentinel;
#[cfg(all(test, target_os = "linux"))]
mod kernel_sentinel_tests;
#[cfg(target_os = "linux")]
pub(crate) mod pcie_keepalive;
#[cfg(all(test, target_os = "linux"))]
mod pcie_keepalive_tests;
#[cfg(feature = "background-monitors")]
mod resource;
#[cfg(target_os = "linux")]
pub(crate) mod silicon_discovery;
#[cfg(feature = "background-monitors")]
mod statistics;

#[cfg(feature = "background-monitors")]
use tracing::info;

#[cfg(feature = "background-monitors")]
use toadstool::RuntimeEngine;

#[cfg(feature = "background-monitors")]
use crate::state::ServerState;

// Re-export for unit tests (only used in #[cfg(test)] mod tests)
#[cfg(all(test, feature = "background-monitors"))]
pub(crate) use cleanup::find_timed_out_execution_ids;
#[cfg(all(test, feature = "background-monitors"))]
pub(crate) use health::perform_health_check;
#[cfg(all(test, feature = "background-monitors"))]
pub(crate) use resource::update_stats_on_tick;

/// Start test-only background monitoring services (resource, health, statistics, cleanup, capability).
///
/// Production services (`pcie_keepalive`, `catalyst_watchdog`, `kernel_sentinel`) are started
/// directly from UniBin / JSON-RPC handler startup — not via this function.
#[cfg(feature = "background-monitors")]
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

    info!("Background monitoring services started");

    // Background tasks will continue running until they're aborted or process exits
    // No need for an infinite loop here - the spawned tasks run independently
}

#[cfg(all(test, feature = "background-monitors"))]
#[path = "tests.rs"]
mod tests;
