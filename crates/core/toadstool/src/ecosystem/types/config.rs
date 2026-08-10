// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem configuration, discovery method options, and the discovered-service alias.

use serde::{Deserialize, Serialize};

use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::Capability;
#[cfg(feature = "runtime")]
use toadstool_common::service_discovery::DiscoveredService;

/// Discovered service instance (type alias for clarity)
pub type ServiceInstance = DiscoveredService;

/// Configuration for ecosystem integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Enable auto-discovery of services
    pub auto_discovery: bool,
    /// Discovery timeout
    pub discovery_timeout: std::time::Duration,
    /// Discovery method to use
    pub discovery_method: DiscoveryMethodConfig,
    /// Required capabilities for operation
    pub required_capabilities: Vec<Capability>,
    /// Optional capabilities for enhanced functionality
    pub optional_capabilities: Vec<Capability>,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_discovery: true,
            discovery_timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
            discovery_method: DiscoveryMethodConfig::Auto,
            // No hardcoded primal names - discover by capability instead
            required_capabilities: vec![],
            optional_capabilities: vec![],
        }
    }
}

impl EcosystemConfig {
    /// Create a new config builder
    pub fn builder() -> EcosystemConfigBuilder {
        EcosystemConfigBuilder::default()
    }
}

/// Builder for `EcosystemConfig` (fluent API)
#[derive(Default)]
pub struct EcosystemConfigBuilder {
    auto_discovery: bool,
    discovery_timeout: std::time::Duration,
    discovery_method: DiscoveryMethodConfig,
    required_capabilities: Vec<Capability>,
    optional_capabilities: Vec<Capability>,
}

impl EcosystemConfigBuilder {
    /// Enable or disable auto-discovery
    pub const fn auto_discovery(mut self, enabled: bool) -> Self {
        self.auto_discovery = enabled;
        self
    }

    /// Set discovery timeout
    pub const fn discovery_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.discovery_timeout = timeout;
        self
    }

    /// Set discovery method
    pub fn discovery_method(mut self, method: DiscoveryMethodConfig) -> Self {
        self.discovery_method = method;
        self
    }

    /// Add a required capability
    pub fn require_capability(mut self, capability: Capability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    /// Add an optional capability
    pub fn optional_capability(mut self, capability: Capability) -> Self {
        self.optional_capabilities.push(capability);
        self
    }

    /// Build the configuration
    pub fn build(self) -> EcosystemConfig {
        EcosystemConfig {
            auto_discovery: self.auto_discovery,
            discovery_timeout: self.discovery_timeout,
            discovery_method: self.discovery_method,
            required_capabilities: self.required_capabilities,
            optional_capabilities: self.optional_capabilities,
        }
    }
}

/// Discovery method configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DiscoveryMethodConfig {
    /// Automatic selection
    #[default]
    Auto,
    /// Environment variables only
    Environment,
    /// mDNS discovery
    Mdns,
    /// Configuration file
    ConfigFile {
        /// Path to the config file.
        path: String,
    },
    /// Registry service
    Registry {
        /// Registry endpoint URL.
        endpoint: String,
    },
}
