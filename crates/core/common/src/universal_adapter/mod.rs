//! Universal Adapter - Capability-Based Primal Discovery
//!
//! ## Philosophy: "Infant Discovery"
//!
//! Each primal is born knowing only itself. ToadStool doesn't know beardog exists.
//! ToadStool knows it needs 'security' capability. At runtime, it discovers WHO
//! provides that capability through the Universal Adapter.
//!
//! ## Core Principles
//!
//! 1. **No Hardcoded Primal Names**: Request capabilities, not specific primals
//! 2. **Runtime Discovery**: All discovery happens at runtime via multiple sources
//! 3. **Self-Knowledge Only**: Each primal knows only itself
//! 4. **Graceful Degradation**: Works without specific primals
//! 5. **Vendor Agnostic**: Works with any discovery mechanism
//!
//! ## Usage
//!
//! ```rust,no_run
//! use toadstool_common::universal_adapter::{UniversalAdapter, CapabilityType, SecurityFeature};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create adapter with auto-discovery
//! let adapter = UniversalAdapter::new().await?;
//!
//! // Request security capability (WHO provides it is discovered at runtime)
//! let security = adapter.request_capability(
//!     CapabilityType::Security {
//!         features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
//!         min_trust_level: TrustLevel::High,
//!     }
//! ).await?;
//!
//! // Use the capability (don't care who provides it)
//! let encrypted = security.encrypt(b"sensitive data")?;
//! # Ok(())
//! # }
//! ```

pub mod capability_types;
pub mod discovery_engine;
pub mod graceful_degradation;
pub mod provider_registry;
pub mod request_builder;

pub use capability_types::*;
pub use discovery_engine::*;
pub use graceful_degradation::*;
pub use provider_registry::*;
pub use request_builder::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ToadStoolResult;

/// Universal Adapter for capability-based primal discovery
///
/// The adapter discovers available capability providers at runtime and matches
/// requests to the best available provider. No primal names are hardcoded.
pub struct UniversalAdapter {
    /// Discovery engine (mDNS, environment, registry, etc.)
    discovery: Arc<DiscoveryEngine>,

    /// Runtime registry of discovered providers
    registry: Arc<RwLock<ProviderRegistry>>,

    /// Graceful degradation strategy
    degradation: Arc<GracefulDegradation>,
}

impl UniversalAdapter {
    /// Create a new Universal Adapter with default discovery sources
    ///
    /// Default sources: mDNS, environment variables, local registry
    ///
    /// # Errors
    ///
    /// Returns error if discovery sources cannot be initialized
    pub async fn new() -> ToadStoolResult<Self> {
        let discovery = Arc::new(DiscoveryEngine::with_defaults()?);
        let registry = Arc::new(RwLock::new(ProviderRegistry::new()));
        let degradation = Arc::new(GracefulDegradation::new());

        let adapter = Self {
            discovery,
            registry,
            degradation,
        };

        // Initial discovery scan
        adapter.discover_providers().await?;

        Ok(adapter)
    }

    /// Create adapter with custom discovery sources
    pub async fn with_sources(sources: Vec<Box<dyn DiscoverySource>>) -> ToadStoolResult<Self> {
        let discovery = Arc::new(DiscoveryEngine::new(sources)?);
        let registry = Arc::new(RwLock::new(ProviderRegistry::new()));
        let degradation = Arc::new(GracefulDegradation::new());

        let adapter = Self {
            discovery,
            registry,
            degradation,
        };

        adapter.discover_providers().await?;

        Ok(adapter)
    }

    /// Discover available capability providers
    ///
    /// Scans all discovery sources and registers found providers
    pub async fn discover_providers(&self) -> ToadStoolResult<usize> {
        let providers = self.discovery.discover_all().await?;

        let mut registry = self.registry.write().await;
        let count = providers.len();

        for provider in providers {
            registry.register(provider)?;
        }

        Ok(count)
    }

    /// Request a capability (the core operation!)
    ///
    /// This is where the magic happens: you request a capability by features,
    /// and the adapter finds WHO can provide it at runtime.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_common::universal_adapter::*;
    /// # async fn example(adapter: &UniversalAdapter) -> Result<(), Box<dyn std::error::Error>> {
    /// // Request security WITHOUT knowing beardog exists
    /// let security = adapter.request_capability(
    ///     CapabilityType::Security {
    ///         features: vec![SecurityFeature::Encryption],
    ///         min_trust_level: TrustLevel::Medium,
    ///     }
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_capability(
        &self,
        capability: CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        // Try to find a provider
        let registry = self.registry.read().await;

        if let Some(provider) = registry.find_best_match(&capability)? {
            return Ok(CapabilityHandle::new(provider, capability));
        }

        // No provider found - try graceful degradation
        self.degradation.handle_missing_capability(capability).await
    }

    /// Get all available capabilities (for introspection)
    pub async fn list_available_capabilities(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let registry = self.registry.read().await;
        Ok(registry.list_capabilities())
    }

    /// Refresh discovery (re-scan for new providers)
    pub async fn refresh(&self) -> ToadStoolResult<usize> {
        self.discover_providers().await
    }

    /// Check if a specific capability is available
    pub async fn has_capability(&self, capability: &CapabilityType) -> bool {
        let registry = self.registry.read().await;
        registry.find_best_match(capability).is_ok()
    }
}

impl Default for UniversalAdapter {
    fn default() -> Self {
        // Note: This is a sync default, so it creates an uninitialized adapter
        // Prefer using UniversalAdapter::new() for production use
        Self {
            discovery: Arc::new(DiscoveryEngine::empty()),
            registry: Arc::new(RwLock::new(ProviderRegistry::new())),
            degradation: Arc::new(GracefulDegradation::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adapter_creation() {
        let adapter = UniversalAdapter::new().await;
        assert!(adapter.is_ok(), "Adapter creation should succeed");
    }

    #[tokio::test]
    async fn test_adapter_default() {
        let adapter = UniversalAdapter::default();
        let caps = adapter.list_available_capabilities().await.unwrap();
        assert_eq!(caps.len(), 0, "Default adapter should have no capabilities");
    }

    #[tokio::test]
    async fn test_capability_check() {
        let adapter = UniversalAdapter::new().await.unwrap();

        let has_security = adapter
            .has_capability(&CapabilityType::Security {
                features: vec![SecurityFeature::Encryption],
                min_trust_level: TrustLevel::Low,
            })
            .await;

        // May or may not have security provider depending on environment
        // Just verify the check completes without error
        // Check that has_security is a valid boolean (this assertion is always true by type system)
        let _ = has_security; // Acknowledge the variable is used
    }

    #[tokio::test]
    async fn test_refresh() {
        let adapter = UniversalAdapter::new().await.unwrap();
        let count = adapter.refresh().await.unwrap();
        // Count is always valid for usize, just verify it completes
        let _ = count;
    }
}
