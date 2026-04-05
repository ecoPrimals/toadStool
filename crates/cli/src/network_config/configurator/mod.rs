// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// Orchestration network configurator for the coordination / service-mesh stack.
///
/// Manages service mesh, DNS discovery, security policies, traffic management,
/// and health monitoring. Which process provides orchestration is discovered at runtime.
///
/// Shorter alias: [`OrchestrationConfigurator`].
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
pub struct OrchestrationNetworkConfigurator {
    /// Network configuration
    pub config: OrchestrationNetworkConfig,
}

/// Short alias for [`OrchestrationNetworkConfigurator`].
pub type OrchestrationConfigurator = OrchestrationNetworkConfigurator;

/// Legacy alias — prefer [`OrchestrationNetworkConfigurator`].
pub type SongbirdNetworkConfigurator = OrchestrationNetworkConfigurator;

// Public API re-exports
impl OrchestrationNetworkConfigurator {
    /// Create a new orchestration network configurator with default configuration
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

impl Default for OrchestrationNetworkConfigurator {
    fn default() -> Self {
        Self::new()
    }
}
