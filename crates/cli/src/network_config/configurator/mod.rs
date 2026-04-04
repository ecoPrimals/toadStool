// SPDX-License-Identifier: AGPL-3.0-only
//! Orchestration Network Configurator
//!
//! Manages network configuration for the orchestration capability:
//! service mesh, DNS discovery, security, traffic management,
//! load balancing, and health monitoring.
//!
//! Primals discover the orchestration service at runtime via capability-based
//! discovery. This configurator is hardware- and service-agnostic.
//!
//! The configurator is organized using extension traits:
//! - `ConfiguratorCore`: Core construction and orchestration
//! - `ServiceMeshExt`: Service mesh configuration
//! - `DiscoveryExt`: DNS and service discovery
//! - `SecurityExt`: Cross-primal security and network policies
//! - `TrafficExt`: Traffic management and load balancing
//! - `ReliabilityExt`: Circuit breakers and health monitoring

use super::types::*;
use toadstool::error::ToadStoolResult;

// Internal modules
mod core;
mod discovery;
mod reliability;
mod security;
mod service_mesh;
mod traffic;

// Re-export traits for internal use
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use core::ConfiguratorCore;
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use discovery::DiscoveryExt;
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use reliability::ReliabilityExt;
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use security::SecurityExt;
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use service_mesh::ServiceMeshExt;
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) use traffic::TrafficExt;

/// Orchestration network configurator (legacy name: SongbirdNetworkConfigurator).
///
/// Manages all aspects of network configuration including service mesh,
/// DNS discovery, security policies, traffic management, and health monitoring.
/// Agnostic to which primal provides orchestration — discovered at runtime.
///
/// Prefer [`OrchestrationConfigurator`] alias for new code.
///
/// # Example
///
/// ```ignore
/// use toadstool_cli::network_config::OrchestrationConfigurator;
///
/// let configurator = OrchestrationConfigurator::new();
/// configurator.apply_configuration().await?;
/// configurator.validate_configuration()?;
/// ```
pub struct SongbirdNetworkConfigurator {
    /// Network configuration
    pub config: SongbirdNetworkConfig,
}

/// Capability-based alias — prefer for new code.
pub type OrchestrationConfigurator = SongbirdNetworkConfigurator;

// Public API re-exports
impl SongbirdNetworkConfigurator {
    /// Create a new Songbird network configurator with default configuration
    pub fn new() -> Self {
        ConfiguratorCore::new()
    }

    /// Apply all network configuration
    pub async fn apply_configuration(&self) -> ToadStoolResult<()> {
        ConfiguratorCore::apply_configuration(self).await
    }

    /// Validate all network configuration
    pub fn validate_configuration(&self) -> ToadStoolResult<()> {
        ConfiguratorCore::validate_configuration(self)
    }
}

impl Default for SongbirdNetworkConfigurator {
    fn default() -> Self {
        Self::new()
    }
}
