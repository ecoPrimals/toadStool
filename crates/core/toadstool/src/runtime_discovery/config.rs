// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery configuration
//!
//! Configuration for mDNS, DNS-SD, intervals, and service limits.

use std::time::Duration;

const DEFAULT_DISCOVERY_INTERVAL_SECS: u64 = 30;
const DEFAULT_SERVICE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_SERVICES: usize = 1000;

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
            discovery_interval: Duration::from_secs(DEFAULT_DISCOVERY_INTERVAL_SECS),
            service_timeout: Duration::from_secs(DEFAULT_SERVICE_TIMEOUT_SECS),
            max_services: DEFAULT_MAX_SERVICES,
        }
    }
}
