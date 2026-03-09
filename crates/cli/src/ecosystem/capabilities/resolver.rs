// SPDX-License-Identifier: AGPL-3.0-only
//! Capability resolver - discovers and resolves capability providers

use crate::Result;
use std::sync::Arc;

use super::registry::{CapabilityRegistry, ServiceProvider};
use super::taxonomy::CapabilityId;
use toadstool_common::infant_discovery::{
    CapabilityDiscovery, DiscoveryEngine, DiscoveryPreferences,
};

/// Capability resolver - bridges between infant discovery and capability registry
pub struct CapabilityResolver {
    /// Discovery engine for finding services
    discovery: Arc<DiscoveryEngine>,

    /// Registry for caching capability providers
    registry: Arc<CapabilityRegistry>,

    /// Enable automatic registration of discovered services
    auto_register: bool,
}

impl CapabilityResolver {
    /// Create a new capability resolver
    pub fn new(discovery: Arc<DiscoveryEngine>, registry: Arc<CapabilityRegistry>) -> Self {
        Self {
            discovery,
            registry,
            auto_register: true,
        }
    }

    /// Resolve a capability to a service provider
    ///
    /// This method:
    /// 1. Checks the registry cache first
    /// 2. Falls back to discovery if not cached
    /// 3. Auto-registers discovered services (if enabled)
    ///
    /// # Example
    /// ```no_run
    /// use toadstool_cli::ecosystem::capabilities::{CapabilityResolver, StandardCapability};
    ///
    /// # async fn example(resolver: CapabilityResolver) -> anyhow::Result<()> {
    /// let provider = resolver
    ///     .resolve(StandardCapability::CryptoSignatureEd25519.id())
    ///     .await?;
    ///     
    /// println!("Found crypto service at: {}", provider.endpoint);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if:
    /// - The capability cannot be found in the registry or via discovery
    /// - Service discovery fails or times out
    /// - Network errors occur during discovery
    #[must_use = "Capability resolution result should be checked"]
    pub async fn resolve(&self, capability: impl Into<CapabilityId>) -> Result<ServiceProvider> {
        let capability = capability.into();

        // Try registry first (cache)
        if let Some(provider) = self.registry.get_best_provider(&capability).await {
            tracing::debug!(
                capability = %capability,
                endpoint = %provider.endpoint,
                "Resolved capability from registry"
            );
            return Ok(provider);
        }

        // Fall back to discovery
        tracing::debug!(
            capability = %capability,
            "Capability not in registry, discovering..."
        );

        let discovered = self
            .discovery
            .discover(capability.as_str())
            .await
            .map_err(|e| {
                crate::CliError::Other(format!(
                    "Failed to discover capability '{capability}': {e:?}"
                ))
            })?;

        let provider = ServiceProvider::from(discovered);

        // Auto-register if enabled
        if self.auto_register {
            self.registry
                .register(capability.clone(), provider.clone())
                .await;
            tracing::info!(
                capability = %capability,
                endpoint = %provider.endpoint,
                "Auto-registered discovered service"
            );
        }

        Ok(provider)
    }

    /// Resolve with preferences
    pub async fn resolve_with_preferences(
        &self,
        capability: impl Into<CapabilityId>,
        preferences: DiscoveryPreferences,
    ) -> Result<ServiceProvider> {
        let capability = capability.into();

        // Try registry first with preference filtering
        let cached_providers = self.registry.get_providers(&capability).await;

        if let Some(provider) = Self::filter_by_preferences(cached_providers, &preferences) {
            return Ok(provider);
        }

        // Fall back to discovery with preferences
        let discovered = self
            .discovery
            .discover_with_preferences(capability.as_str(), preferences)
            .await
            .map_err(|e| {
                crate::CliError::Other(format!(
                    "Failed to discover capability '{capability}': {e:?}"
                ))
            })?;

        let provider = ServiceProvider::from(discovered);

        if self.auto_register {
            self.registry.register(capability, provider.clone()).await;
        }

        Ok(provider)
    }

    /// Get all providers for a capability
    pub async fn resolve_all(
        &self,
        capability: impl Into<CapabilityId>,
    ) -> Result<Vec<ServiceProvider>> {
        let capability = capability.into();

        // Get from registry
        let mut providers = self.registry.get_providers(&capability).await;

        // If empty, try discovery
        if providers.is_empty() {
            if let Ok(discovered) = self.discovery.discover(capability.as_str()).await {
                let provider = ServiceProvider::from(discovered);

                if self.auto_register {
                    self.registry.register(capability, provider.clone()).await;
                }

                providers.push(provider);
            }
        }

        Ok(providers)
    }

    /// Refresh a capability (re-discover and update registry)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Service re-discovery fails or times out
    /// - Network errors occur during discovery
    /// - The capability cannot be found
    #[must_use = "Capability refresh result should be checked"]
    pub async fn refresh(&self, capability: impl Into<CapabilityId>) -> Result<ServiceProvider> {
        let capability = capability.into();

        // Force discovery (bypass cache)
        let discovered = self
            .discovery
            .discover(capability.as_str())
            .await
            .map_err(|e| {
                crate::CliError::Other(format!(
                    "Failed to refresh capability '{capability}': {e:?}"
                ))
            })?;

        let provider = ServiceProvider::from(discovered);

        // Update registry
        self.registry.register(capability, provider.clone()).await;

        Ok(provider)
    }

    /// Configure auto-registration
    pub fn set_auto_register(&mut self, enabled: bool) {
        self.auto_register = enabled;
    }

    /// Filter providers by preferences
    fn filter_by_preferences(
        providers: Vec<ServiceProvider>,
        preferences: &DiscoveryPreferences,
    ) -> Option<ServiceProvider> {
        providers.into_iter().find(|p| {
            // Check protocol requirements
            if !preferences.required_protocols.is_empty() {
                let has_required = preferences
                    .required_protocols
                    .iter()
                    .any(|req| p.protocols.contains(req));

                if !has_required {
                    return false;
                }
            }

            // Check health requirement
            if p.health < preferences.min_health {
                return false;
            }

            // Check locality preference
            if preferences.prefer_local
                && !p
                    .endpoint
                    .contains(toadstool_common::constants::DEFAULT_HOSTNAME)
                && !p
                    .endpoint
                    .contains(toadstool_common::constants::LOCALHOST_IPV4)
            {
                return false;
            }

            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::capabilities::taxonomy::StandardCapability;
    use toadstool_common::infant_discovery::{
        DiscoveryPreferences, ServiceHealth, ServiceMetadata,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resolve_from_registry() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        // Pre-populate registry
        let capability = StandardCapability::CryptoSignatureEd25519.id();
        let provider = ServiceProvider {
            endpoint: "http://localhost:8081".to_string(),
            protocols: vec!["http".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: std::time::Instant::now(),
            priority: 80,
        };

        registry.register(capability.clone(), provider).await;

        // Resolve should find it
        let resolved = resolver.resolve(capability).await.unwrap();
        assert_eq!(resolved.endpoint, "http://localhost:8081");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resolve_with_preferences_from_registry() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        let capability = StandardCapability::CryptoSignatureEd25519.id();
        let provider = ServiceProvider {
            endpoint: "unix:///var/run/crypto.sock".to_string(),
            protocols: vec!["unix".to_string(), "jsonrpc".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: std::time::Instant::now(),
            priority: 90,
        };

        registry.register(capability.clone(), provider).await;

        let prefs = DiscoveryPreferences {
            required_protocols: vec!["unix".to_string()],
            min_health: ServiceHealth::Healthy,
            prefer_local: false,
            timeout: None,
            preferred_sources: vec![],
        };

        let resolved = resolver
            .resolve_with_preferences(capability, prefs)
            .await
            .unwrap();
        assert!(resolved.endpoint.contains("unix"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resolve_all_from_registry() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        let capability = StandardCapability::StorageObjectS3.id();
        let provider = ServiceProvider {
            endpoint: "http://localhost:9000".to_string(),
            protocols: vec!["http".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: std::time::Instant::now(),
            priority: 50,
        };

        registry.register(capability.clone(), provider).await;

        let providers = resolver.resolve_all(capability).await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].endpoint, "http://localhost:9000");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resolve_all_empty_when_not_found() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        let capability = StandardCapability::CryptoKeyGeneration.id();
        let providers = resolver.resolve_all(capability).await.unwrap();
        assert!(providers.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_refresh_fails_when_no_discovery() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        let capability = StandardCapability::CryptoSignatureEd25519.id();
        let result = resolver.refresh(capability).await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_set_auto_register() {
        let discovery = Arc::new(DiscoveryEngine::new());
        let registry = Arc::new(CapabilityRegistry::new());
        let mut resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

        resolver.set_auto_register(false);
        // Just verify it doesn't panic - we can't easily test the effect
        // without mocking discovery
        drop(resolver);
    }
}
