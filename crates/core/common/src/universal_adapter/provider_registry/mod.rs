// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provider Registry - Runtime Catalog of Capability Providers
//!
//! Maintains a runtime registry of discovered providers and matches
//! capability requests to the best available provider.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::capability_types::CapabilityInfo;

mod lifecycle;
mod lookup;
mod registration;
#[cfg(test)]
mod tests;

/// Runtime registry of capability providers
pub struct ProviderRegistry {
    /// Providers indexed by ID
    pub(super) providers: HashMap<String, RegisteredProvider>,

    /// Provider health check interval (for future use)
    #[expect(dead_code, reason = "reserved for periodic health checks")]
    pub(super) health_check_interval: Duration,
}

/// Registered provider with metadata
pub(super) struct RegisteredProvider {
    pub(super) info: CapabilityInfo,
    pub(super) registered_at: Instant,
    pub(super) last_health_check: Option<Instant>,
    pub(super) request_count: u64,
    pub(super) failure_count: u64,
}
