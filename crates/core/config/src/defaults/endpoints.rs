// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Self-Configuration Endpoints
//!
//! **Philosophy**: ToadStool should only have knowledge about its own API endpoint.
//! Other primals must be discovered at runtime using `BiomeOSClient` or `RuntimeDiscovery`.
//!
//! # Migration from Deprecated Endpoints
//!
//! The following endpoint helpers have been REMOVED to enforce infant discovery:
//! - coordination — use `BiomeOSClient::get_coordination_provider().await?.endpoint`
//! - security — use `BiomeOSClient::get_security_provider().await?.endpoint`
//! - storage — use `BiomeOSClient::get_storage_provider().await?.endpoint`
//! - intelligence / AI — use `BiomeOSClient::get_ai_provider().await?.endpoint`
//!
//! # Example
//!
//! ```rust,ignore
//! // OLD (hardcoded - REMOVED):
//! // let url = defaults::endpoints::security_endpoint();
//!
//! // NEW (discovered):
//! use toadstool::biomeos_integration::BiomeOSClient;
//!
//! let biomeos = BiomeOSClient::connect().await?;
//! let security = biomeos.get_security_provider().await?;
//! let url = security.endpoint; // Discovered at runtime!
//! ```

use toadstool_common::constants::network::DEFAULT_HOSTNAME;

use super::network;
use crate::ports::capability_fallback;

/// HTTP base URL for coordination capability cold-start bootstrap using the default hostname.
///
/// Default port is [`capability_fallback::COORDINATION`]. Override outbound URL with
/// `TOADSTOOL_COORDINATION_ENDPOINT`, `TOADSTOOL_COORDINATION_SERVICE_URL`, or discovery — not this helper.
#[must_use]
pub fn coordination_localhost_bootstrap_url() -> String {
    format!(
        "http://{}:{}",
        DEFAULT_HOSTNAME,
        capability_fallback::COORDINATION
    )
}

/// Same as [`coordination_localhost_bootstrap_url`] but with loopback IPv4 (`127.0.0.1`), for clients and tests
/// that require numeric loopback.
#[must_use]
pub fn coordination_loopback_bootstrap_url() -> String {
    format!(
        "http://{}:{}",
        network::LOCALHOST,
        capability_fallback::COORDINATION
    )
}

/// Default API endpoint (port 0 = OS-assigned, use discovery for actual port)
#[must_use]
pub fn api() -> String {
    format!("http://{}:{}", DEFAULT_HOSTNAME, network::API_PORT)
}
