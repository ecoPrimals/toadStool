// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery scan results.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::endpoint::ServiceEndpoint;

/// Result of a service discovery scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Discovered service endpoints
    pub services: Vec<ServiceEndpoint>,
    /// How long the scan took
    pub scan_duration: Duration,
    /// Total number of services found
    pub total_discovered: usize,
    /// Number cryptographically verified
    pub verified_count: usize,
}
