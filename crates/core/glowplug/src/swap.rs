// SPDX-License-Identifier: AGPL-3.0-or-later

//! Personality swap orchestration.
//!
//! The swap lifecycle generalizes coralReef's GPU personality swap into a
//! hardware-agnostic sequence:
//!
//! 1. **Quiesce** — drain in-flight operations, wait for quiescence
//! 2. **Persist** — snapshot device-specific state to ember's metadata store
//! 3. **Drop** — release the current exclusive handle
//! 4. **Delegate** — ask ember to perform the actual driver bind/unbind
//! 5. **Reacquire** — get the new handle from ember
//! 6. **Restore** — replay persisted state onto the new personality
//! 7. **Health** — verify the device is healthy in its new personality
//!
//! Each hardware class implements [`SwapExecutor`] for steps 3-5 (the
//! bus-specific part). The orchestration framework (steps 1-2, 6-7) is
//! shared.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::device_id::DeviceId;

/// Result of a personality swap attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapObservation {
    /// Device that was swapped.
    pub device_id: String,
    /// Personality before the swap.
    pub from: String,
    /// Personality after the swap (or attempted target on failure).
    pub to: String,
    /// Whether the swap succeeded.
    pub success: bool,
    /// How long the swap took.
    pub duration: Duration,
    /// Optional error message if the swap failed.
    pub error: Option<String>,
    /// Optional diagnostic detail (hardware-specific).
    pub detail: Option<serde_json::Value>,
}

/// Executes the bus-specific portion of a personality swap.
///
/// The orchestrator calls this after quiescing and persisting state,
/// and before restoring state and health-checking.
#[async_trait::async_trait]
pub trait SwapExecutor: Send + Sync + fmt::Debug {
    /// Error type for swap operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Perform the actual driver unbind/rebind for this device.
    ///
    /// The implementation should:
    /// 1. Drop the current exclusive handle (if any)
    /// 2. Trigger the kernel to switch drivers (sysfs bind/unbind, etc.)
    /// 3. Acquire the new exclusive handle
    ///
    /// # Errors
    ///
    /// Returns an error if the swap fails at any step.
    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error>;

    /// Release a device back to unbound state without swapping to another
    /// personality.
    ///
    /// # Errors
    ///
    /// Returns an error if the release fails.
    async fn release(&self, device: &DeviceId) -> Result<(), Self::Error>;
}

/// Orchestrates the full swap lifecycle.
///
/// Wraps a [`SwapExecutor`] with quiescence, persistence, restoration, and
/// health checking. This is the high-level API that the ecosystem calls.
#[derive(Debug)]
pub struct SwapOrchestrator<E: SwapExecutor> {
    executor: E,
    quiescence_timeout: Duration,
}

impl<E: SwapExecutor> SwapOrchestrator<E> {
    /// Create a new orchestrator wrapping a bus-specific executor.
    #[must_use]
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            quiescence_timeout: Duration::from_secs(5),
        }
    }

    /// Set the quiescence timeout.
    #[must_use]
    pub const fn with_quiescence_timeout(mut self, timeout: Duration) -> Self {
        self.quiescence_timeout = timeout;
        self
    }

    /// Access the underlying executor.
    #[must_use]
    pub const fn executor(&self) -> &E {
        &self.executor
    }

    /// The configured quiescence timeout.
    #[must_use]
    pub const fn quiescence_timeout(&self) -> Duration {
        self.quiescence_timeout
    }
}
