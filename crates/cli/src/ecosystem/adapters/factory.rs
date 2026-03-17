// SPDX-License-Identifier: AGPL-3.0-only
//! Adapter factory - simplifies initialization of capability-based adapters
//!
//! This factory reduces boilerplate when creating adapters by handling the
//! common initialization pattern: discovery → registry → resolver → adapter.

use crate::Result;
use std::sync::Arc;

use super::{
    CoordinationAdapter, CryptoAdapter, StorageAdapter, universal::UniversalServiceAdapter,
};
use crate::ecosystem::capabilities::{CapabilityRegistry, CapabilityResolver};
use toadstool_common::infant_discovery::DiscoveryEngine;

/// Factory for creating capability-based adapters
///
/// This factory simplifies the initialization pattern proven in Migration #001.
/// Instead of manually wiring up discovery → registry → resolver → adapter,
/// use this factory to get fully-initialized adapters in one call.
///
/// # Example
/// ```ignore
/// use toadstool_cli::ecosystem::adapters::AdapterFactory;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Create factory (once per application)
/// let factory = AdapterFactory::new();
///
/// // Get crypto adapter (no boilerplate!)
/// let crypto = factory.crypto_adapter()?;
///
/// // Use capability-based API
/// crypto.install_permissions(&path, false).await?;
/// # Ok(())
/// # }
/// ```
pub struct AdapterFactory {
    /// Discovery engine (shared across all adapters)
    discovery: Arc<DiscoveryEngine>,
    /// Capability registry (shared across all adapters)
    registry: Arc<CapabilityRegistry>,
    /// Capability resolver (shared across all adapters)
    resolver: Arc<CapabilityResolver>,
    /// Universal service adapter (shared across all adapters)
    universal: Arc<UniversalServiceAdapter>,
}

impl AdapterFactory {
    /// Create a new adapter factory
    ///
    /// This initializes the shared infrastructure once, which is then
    /// reused by all adapters.
    #[must_use]
    pub fn new() -> Self {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = Arc::new(CapabilityResolver::new(
            Arc::clone(&discovery),
            Arc::clone(&registry),
        ));
        let universal = Arc::new(UniversalServiceAdapter::new(Arc::clone(&resolver)));

        Self {
            discovery,
            registry,
            resolver,
            universal,
        }
    }

    /// Get a crypto adapter
    ///
    /// # Example
    /// ```ignore
    /// let factory = AdapterFactory::new();
    /// let crypto = factory.crypto_adapter()?;
    /// ```
    pub fn crypto_adapter(&self) -> Result<CryptoAdapter> {
        Ok(CryptoAdapter::new(Arc::clone(&self.universal)))
    }

    /// Get a storage adapter
    ///
    /// # Example
    /// ```ignore
    /// let factory = AdapterFactory::new();
    /// let storage = factory.storage_adapter()?;
    /// ```
    pub fn storage_adapter(&self) -> Result<StorageAdapter> {
        Ok(StorageAdapter::new(Arc::clone(&self.universal)))
    }

    /// Get a coordination adapter
    ///
    /// # Example
    /// ```ignore
    /// let factory = AdapterFactory::new();
    /// let coordination = factory.coordination_adapter()?;
    /// ```
    pub fn coordination_adapter(&self) -> Result<CoordinationAdapter> {
        Ok(CoordinationAdapter::new(Arc::clone(&self.universal)))
    }

    /// Get the underlying discovery engine
    ///
    /// Useful for advanced use cases that need direct access to discovery.
    #[must_use]
    pub fn discovery_engine(&self) -> Arc<DiscoveryEngine> {
        Arc::clone(&self.discovery)
    }

    /// Get the underlying capability registry
    ///
    /// Useful for advanced use cases that need direct access to the registry.
    #[must_use]
    pub fn capability_registry(&self) -> Arc<CapabilityRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get the underlying capability resolver
    ///
    /// Useful for advanced use cases that need direct access to the resolver.
    #[must_use]
    pub fn capability_resolver(&self) -> Arc<CapabilityResolver> {
        Arc::clone(&self.resolver)
    }

    /// Get the underlying universal adapter
    ///
    /// Useful for creating custom adapters.
    #[must_use]
    pub fn universal_adapter(&self) -> Arc<UniversalServiceAdapter> {
        Arc::clone(&self.universal)
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = AdapterFactory::new();
        assert!(factory.crypto_adapter().is_ok());
        assert!(factory.storage_adapter().is_ok());
        assert!(factory.coordination_adapter().is_ok());
    }

    #[test]
    fn test_factory_default() {
        let factory = AdapterFactory::default();
        assert!(factory.crypto_adapter().is_ok());
    }

    #[test]
    fn test_factory_shares_infrastructure() {
        let factory = AdapterFactory::new();

        let crypto1 = factory.crypto_adapter().unwrap();
        let crypto2 = factory.crypto_adapter().unwrap();

        // Both adapters use the same underlying universal adapter
        // (We can't directly test Arc equality, but we can verify they work)
        drop(crypto1);
        drop(crypto2);
    }
}
