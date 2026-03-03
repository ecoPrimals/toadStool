// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Self-Configuration Endpoints
//!
//! **Philosophy**: ToadStool should only have knowledge about its own API endpoint.
//! Other primals must be discovered at runtime using `BiomeOSClient` or `RuntimeDiscovery`.
//!
//! # Migration from Deprecated Endpoints
//!
//! The following endpoint helpers have been REMOVED to enforce infant discovery:
//! - `songbird()` - Use `BiomeOSClient::get_coordination_provider().await?.endpoint`
//! - `beardog()` - Use `BiomeOSClient::get_security_provider().await?.endpoint`
//! - `nestgate()` - Use `BiomeOSClient::get_storage_provider().await?.endpoint`
//! - `squirrel()` - Use `BiomeOSClient::get_ai_provider().await?.endpoint`
//!
//! # Example
//!
//! ```rust,ignore
//! // OLD (hardcoded - REMOVED):
//! // let url = defaults::endpoints::beardog();
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

/// Default API endpoint (port 0 = OS-assigned, use discovery for actual port)
#[must_use]
pub fn api() -> String {
    format!("http://{}:{}", DEFAULT_HOSTNAME, network::API_PORT)
}

/// Default cloud endpoint
///
/// Port 0 = OS-assigned. In production, prefer capability-based discovery.
#[deprecated(note = "Use capability-based discovery via discover_or_fallback() instead")]
#[must_use]
pub fn cloud() -> String {
    format!("http://{}:{}", DEFAULT_HOSTNAME, network::API_PORT)
}
