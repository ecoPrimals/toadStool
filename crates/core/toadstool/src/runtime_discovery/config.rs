// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery configuration
//!
//! Configuration for mDNS, DNS-SD, intervals, and service limits.

use std::time::Duration;

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    pub enable_mdns: bool,

    /// Enable DNS-SD discovery
    pub enable_dns_sd: bool,

    /// Discovery interval
    pub discovery_interval: Duration,

    /// Service timeout (mark as stale)
    pub service_timeout: Duration,

    /// Maximum services to track
    pub max_services: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_dns_sd: true,
            discovery_interval: Duration::from_secs(30),
            service_timeout: Duration::from_secs(300), // 5 minutes
            max_services: 1000,
        }
    }
}
