// SPDX-License-Identifier: AGPL-3.0-or-later
//! Orchestration error types

use thiserror::Error;

/// Errors that can occur during workload orchestration
#[derive(Debug, Error)]
pub enum OrchestrationError {
    /// No compute substrates are available.
    #[error("No substrates available")]
    NoSubstrates,

    /// All substrates failed to execute the workload.
    #[error("All substrates failed")]
    AllSubstratesFailed,

    /// Operation count must be greater than zero.
    #[error("Operation count must be > 0")]
    InvalidOperationCount,

    /// Substrate-specific error.
    #[error("Substrate error: {0}")]
    Substrate(String),

    /// Requested resource is unavailable.
    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),

    /// Resource quota exceeded.
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    /// Guest load exceeds threshold — workload yielded to owner.
    ///
    /// Returned when `max_guest_load` policy is active and the current
    /// GPU-bound workload count exceeds `max_concurrent_gpu`. The yield
    /// strategy determines the action: `Queue` defers, `Reject` fails
    /// immediately, `DeferUntilPowerCycle` waits for a host power-cycle
    /// window to complete.
    #[error("Guest load exceeded: {0}")]
    GuestLoadExceeded(String),

    /// Internal lock was poisoned by a prior panic.
    #[error("Internal lock poisoned: {0}")]
    LockPoisoned(String),
}
