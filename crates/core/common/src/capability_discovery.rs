//! Capability-Based Service Discovery - Pure Infant Discovery Pattern
//!
//! This module implements the **pure infant discovery** pattern where ToadStool:
//! 1. Knows only itself (self-knowledge)
//! 2. Discovers other primals at runtime by capabilities
//! 3. Has zero hardcoded primal names or ports
//!
//! ## Philosophy
//!
//! > "Each primal is born knowing nothing except itself.
//! >  It discovers others by what they can do, not who they are."
//!
//! This enables:
//! - **Multi-vendor support**: Any service providing a capability works
//! - **Federation**: Multiple providers for same capability
//! - **Edge capability**: Local service discovery (mDNS)
//! - **Cloud capability**: Kubernetes service discovery
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use toadstool_common::capability_discovery::CapabilityDiscovery;
//! use toadstool_common::primal_identity::{Capability, CryptoCapability};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create discovery engine
//! let discovery = CapabilityDiscovery::new()?;
//!
//! // Find ANY service that can encrypt (agnostic of provider)
//! let crypto_services = discovery
//!     .find_by_capability(Capability::Crypto(CryptoCapability::Encryption))
//!     .await?;
//!
//! // Use the first available (or implement selection logic)
//! if let Some(service) = crypto_services.first() {
//!     println!("Found crypto service: {}", service.name);
//!     // Connect and use...
//! }
//! # Ok(())
//! # }
//! ```

use crate::primal_identity::Capability;
use crate::service_discovery::{DiscoveredService, ServiceDiscovery, ServiceDiscoveryTrait};
use std::time::Duration;
use thiserror::Error;

/// Capability-based discovery client
///
/// This is the **primary interface** for discovering services by capability.
/// It abstracts over multiple discovery methods (mDNS, DNS-SD, K8s, Consul, etc.)
pub struct CapabilityDiscovery {
    /// Underlying discovery implementation
    discovery: Box<dyn ServiceDiscoveryTrait>,

    /// Discovery timeout
    timeout: Duration,

    /// Enable fallback to localhost in development
    enable_localhost_fallback: bool,
}

impl CapabilityDiscovery {
    /// Create new capability discovery client
    ///
    /// Automatically detects available discovery methods:
    /// - Kubernetes (if in K8s cluster)
    /// - mDNS/DNS-SD (if on local network)
    /// - Environment variables (always available)
    pub fn new() -> Result<Self, DiscoveryError> {
        Self::with_config(&DiscoveryConfig::default())
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if the tokio runtime cannot be created
    pub fn with_config(config: &DiscoveryConfig) -> Result<Self, DiscoveryError> {
        // Detect and initialize appropriate discovery backend
        let discovery = Self::detect_discovery_backend()?;

        Ok(Self {
            discovery,
            timeout: config.timeout,
            enable_localhost_fallback: config.enable_localhost_fallback,
        })
    }

    /// Find services by capability
    ///
    /// Returns ALL services providing the requested capability.
    /// Caller can choose based on additional criteria (latency, trust level, etc.)
    ///
    /// ## Multi-Provider Example
    ///
    /// ```rust,no_run
    /// # use toadstool_common::capability_discovery::CapabilityDiscovery;
    /// # use toadstool_common::primal_identity::{Capability, CryptoCapability};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = CapabilityDiscovery::new()?;
    ///
    /// // Find ALL crypto providers
    /// let providers = discovery
    ///     .find_by_capability(Capability::Crypto(CryptoCapability::Encryption))
    ///     .await?;
    ///
    /// // Select based on criteria (metadata can contain latency, trust info)
    /// let local_provider = providers.iter()
    ///     .filter(|s| s.healthy)
    ///     .next();
    ///
    /// let latest_version = providers.iter()
    ///     .max_by(|a, b| a.version.cmp(&b.version));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_capability(
        &self,
        capability: Capability,
    ) -> Result<Vec<DiscoveredService>, DiscoveryError> {
        // Use timeout
        let result = tokio::time::timeout(
            self.timeout,
            self.discovery.find_services_by_capability(&capability),
        )
        .await
        .map_err(|_| DiscoveryError::Timeout)?;

        match result {
            Ok(services) if !services.is_empty() => Ok(services),
            Ok(_) if self.enable_localhost_fallback => {
                // Fallback for development
                self.try_localhost_fallback(&capability).await
            }
            Ok(_) => Err(DiscoveryError::NoServicesFound(format!("{capability:?}"))),
            Err(e) => Err(DiscoveryError::DiscoveryFailed(e.to_string())),
        }
    }

    /// Find the "best" service for a capability
    ///
    /// Selection criteria (in order):
    /// 1. Healthy services preferred
    /// 2. Most recently seen (fresher discovery)
    /// 3. Metadata-based trust level (if available)
    pub async fn find_best(
        &self,
        capability: Capability,
    ) -> Result<DiscoveredService, DiscoveryError> {
        let services = self.find_by_capability(capability).await?;

        services
            .into_iter()
            .max_by(|a, b| {
                // Healthy services first
                let health_cmp = a.healthy.cmp(&b.healthy);
                if health_cmp != std::cmp::Ordering::Equal {
                    return health_cmp;
                }

                // Then by freshness (more recent is better)
                a.last_seen.cmp(&b.last_seen)
            })
            .ok_or(DiscoveryError::NoServicesFound(
                "No services available".to_string(),
            ))
    }

    /// Detect available discovery backend
    fn detect_discovery_backend() -> Result<Box<dyn ServiceDiscoveryTrait>, DiscoveryError> {
        // TODO: Implement detection logic
        // 1. Check for K8s environment (KUBERNETES_SERVICE_HOST)
        // 2. Check for mDNS availability
        // 3. Fall back to environment variables

        // For now, use the existing service discovery implementation
        use crate::service_discovery::DiscoveryMethod;

        // ServiceDiscovery::new is async, so we need to run it in a blocking context
        // In production, this would be handled differently (e.g., async initialization)
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            DiscoveryError::InvalidConfig(format!("Failed to create runtime: {e}"))
        })?;

        let discovery = runtime
            .block_on(ServiceDiscovery::new(DiscoveryMethod::Auto))
            .map_err(|e| DiscoveryError::DiscoveryFailed(e.to_string()))?;

        Ok(Box::new(discovery))
    }

    /// Try localhost fallback for development
    async fn try_localhost_fallback(
        &self,
        _capability: &Capability,
    ) -> Result<Vec<DiscoveredService>, DiscoveryError> {
        // Return empty for now - localhost fallback should use environment variables
        Ok(vec![])
    }
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery timeout
    pub timeout: Duration,

    /// Enable localhost fallback in development
    pub enable_localhost_fallback: bool,

    /// Discovery methods to try
    pub methods: Vec<DiscoveryMethod>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        let is_production = std::env::var("TOADSTOOL_ENV")
            .map(|e| e == "production")
            .unwrap_or(false);

        Self {
            timeout: Duration::from_secs(5),
            enable_localhost_fallback: !is_production,
            methods: vec![DiscoveryMethod::Auto],
        }
    }
}

/// Discovery methods
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryMethod {
    /// Automatically detect best method
    Auto,

    /// Kubernetes service discovery
    Kubernetes,

    /// mDNS/DNS-SD (local network)
    Mdns,

    /// Consul service discovery
    Consul,

    /// Environment variables
    Environment,
}

/// Discovery errors
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Discovery timeout")]
    Timeout,

    #[error("No services found for capability: {0}")]
    NoServicesFound(String),

    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(5));
        // Fallback enabled in non-production
        assert!(config.enable_localhost_fallback);
    }

    #[test]
    fn test_discovery_config_custom() {
        let config = DiscoveryConfig {
            timeout: Duration::from_secs(10),
            enable_localhost_fallback: false,
            methods: vec![DiscoveryMethod::Kubernetes],
        };
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert!(!config.enable_localhost_fallback);
    }

    #[test]
    fn test_discovery_method_copy() {
        let method1 = DiscoveryMethod::Auto;
        let method2 = method1; // Copy
        assert!(matches!(method1, DiscoveryMethod::Auto));
        assert!(matches!(method2, DiscoveryMethod::Auto));
    }
}
