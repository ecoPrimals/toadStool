//! Songbird Network Configurator
//!
//! This module provides network configuration management for Songbird,
//! including service mesh, DNS discovery, security, traffic management,
//! load balancing, and health monitoring.
//!
//! The configurator is organized using extension traits for better code organization:
//! - `ConfiguratorCore`: Core construction and orchestration
//! - `ServiceMeshExt`: Service mesh configuration
//! - `DiscoveryExt`: DNS and service discovery
//! - `SecurityExt`: Cross-primal security and network policies
//! - `TrafficExt`: Traffic management and load balancing
//! - `ReliabilityExt`: Circuit breakers and health monitoring

use super::types::*;
use reqwest::Client;
use toadstool::error::ToadStoolResult;

// Internal modules
mod core;
mod discovery;
mod reliability;
mod security;
mod service_mesh;
mod traffic;

// Re-export traits for internal use
pub(crate) use core::ConfiguratorCore;
pub(crate) use discovery::DiscoveryExt;
pub(crate) use reliability::ReliabilityExt;
pub(crate) use security::SecurityExt;
pub(crate) use service_mesh::ServiceMeshExt;
pub(crate) use traffic::TrafficExt;

/// Songbird network configurator
///
/// Manages all aspects of Songbird network configuration including service mesh,
/// DNS discovery, security policies, traffic management, and health monitoring.
///
/// # Example
///
/// ```ignore
/// use toadstool_cli::network_config::SongbirdNetworkConfigurator;
///
/// let configurator = SongbirdNetworkConfigurator::new();
/// configurator.apply_configuration().await?;
/// configurator.validate_configuration()?;
/// ```
pub struct SongbirdNetworkConfigurator {
    /// HTTP client for making configuration requests
    #[allow(dead_code)]
    pub(crate) client: Client,

    /// Network configuration
    pub config: SongbirdNetworkConfig,
}

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
