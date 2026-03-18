// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery state and statistics
//!
//! Internal state and public statistics for the discovery engine.

/// Discovery state (internal)
#[derive(Debug)]
pub(super) struct DiscoveryState {
    /// Is discovery running?
    pub(super) running: bool,

    /// Last discovery time
    #[allow(dead_code, reason = "timestamped for future stale-service reporting")]
    pub(super) last_discovery: Option<std::time::SystemTime>,

    /// Discovery statistics
    pub(super) stats: DiscoveryStats,
}

/// Discovery statistics
#[derive(Debug, Default)]
pub struct DiscoveryStats {
    /// Total discoveries
    pub total_discovered: u64,

    /// Currently active services
    pub active_services: usize,

    /// Services that timed out
    pub timeouts: u64,
}
