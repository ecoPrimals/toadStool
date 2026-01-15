//! # Service Discovery
//!
//! Capability-based service discovery for the ecosystem.
//!
//! ## Philosophy
//!
//! - **Capability-First**: Discover by what services can do
//! - **Strategy Pattern**: Pluggable discovery methods
//! - **Zero Hardcoding**: No primal names or ports in discovery logic
//! - **Auto-Fallback**: Tries multiple methods automatically
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Modern approach: Find by capability
//! let coordinator = DiscoveryManager::new(&discovery_client).await?;
//! let service = coordinator.find_by_capability(
//!     Capability::Coordination(...)
//! ).await?;
//!
//! // Automatic discovery of all required capabilities
//! let services = coordinator.discover_all_required(&config).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::{DiscoveredService, ServiceDiscovery};

use super::types::{EcosystemConfig, ServiceInstance};

/// Discovery manager for finding services by capability
pub struct DiscoveryManager {
    /// Service discovery client
    discovery_client: Arc<ServiceDiscovery>,
    /// Cache of discovered services (keyed by service ID)
    cache: Arc<RwLock<HashMap<String, DiscoveredService>>>,
    /// Cache of capability → service mappings
    capability_cache: Arc<RwLock<HashMap<String, Vec<String>>>>, // capability key → service IDs
}

impl DiscoveryManager {
    /// Create a new discovery manager
    pub fn new(discovery_client: Arc<ServiceDiscovery>) -> Self {
        Self {
            discovery_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            capability_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Find a service by capability
    ///
    /// This is the **modern, recommended approach** for finding services.
    pub async fn find_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<DiscoveredService> {
        info!("🔍 Finding service with capability: {:?}", capability);

        // Check capability cache first
        let capability_key = format!("{:?}", capability);
        {
            let cap_cache = self.capability_cache.read().await;
            if let Some(service_ids) = cap_cache.get(&capability_key) {
                if let Some(service_id) = service_ids.first() {
                    let cache = self.cache.read().await;
                    if let Some(service) = cache.get(service_id) {
                        debug!("✅ Found service in cache: {}", service.name);
                        return Ok(service.clone());
                    }
                }
            }
        }

        // Not in cache - discover via client
        let service = self
            .discovery_client
            .find_service_by_capability(capability.clone())
            .await
            .map_err(|e| ToadStoolError::not_found(format!("No service found: {e}")))?;

        // Update caches
        self.add_to_cache(service.clone(), Some(capability_key))
            .await;

        info!("✅ Found service: {} ({})", service.name, service.id);
        Ok(service)
    }

    /// Discover all services for required capabilities
    pub async fn discover_all_required(
        &self,
        config: &EcosystemConfig,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        info!("🔍 Discovering all required services");

        let mut discovered = Vec::new();
        let mut missing_required = Vec::new();

        // Discover required capabilities
        for capability in &config.required_capabilities {
            match self.find_by_capability(capability.clone()).await {
                Ok(service) => {
                    info!("✅ Found required capability: {:?}", capability);
                    discovered.push(service);
                }
                Err(e) => {
                    warn!("❌ Missing required capability: {:?} - {}", capability, e);
                    missing_required.push(capability.clone());
                }
            }
        }

        // Check if any required capabilities are missing
        if !missing_required.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "Missing required capabilities: {:?}",
                missing_required
            )));
        }

        // Discover optional capabilities (failures are OK)
        for capability in &config.optional_capabilities {
            if let Ok(service) = self.find_by_capability(capability.clone()).await {
                info!("✅ Found optional capability: {:?}", capability);
                discovered.push(service);
            } else {
                debug!("Optional capability not found: {:?}", capability);
            }
        }

        info!("✅ Discovered {} services total", discovered.len());
        Ok(discovered)
    }

    /// Discover services via environment (explicit configuration)
    ///
    /// This uses environment variables to explicitly configure service endpoints.
    /// This is acceptable because it's **explicit configuration**, not hardcoding.
    pub async fn discover_via_environment(&self) -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Discovering services via environment configuration");

        // Environment-based discovery is handled by the discovery client
        // We just need to query for known capabilities and let the client
        // use environment variables to find them

        let mut discovered = Vec::new();

        // Common capabilities to check for
        // Note: Capability requires specific enum variants - using NativeExecution as example
        use toadstool_common::primal_identity::ComputeCapability;
        let common_capabilities = vec![Capability::Compute(ComputeCapability::NativeExecution)];

        for capability in &common_capabilities {
            if let Ok(service) = self.find_by_capability(capability.clone()).await {
                discovered.push(service);
            }
        }

        info!(
            "✅ Environment discovery found {} services",
            discovered.len()
        );
        Ok(discovered)
    }

    /// Get a service by ID from cache
    pub async fn get_cached_service(&self, service_id: &str) -> Option<DiscoveredService> {
        let cache = self.cache.read().await;
        cache.get(service_id).cloned()
    }

    /// Get all cached services
    pub async fn get_all_cached(&self) -> Vec<DiscoveredService> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Check if a capability is available (cached or discoverable)
    pub async fn is_capability_available(&self, capability: &Capability) -> bool {
        self.find_by_capability(capability.clone()).await.is_ok()
    }

    /// Clear the discovery cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let mut cap_cache = self.capability_cache.write().await;
        cache.clear();
        cap_cache.clear();
        info!("🗑️  Discovery cache cleared");
    }

    /// Add a service to the cache
    async fn add_to_cache(&self, service: DiscoveredService, capability_key: Option<String>) {
        let mut cache = self.cache.write().await;
        // ✅ OPTIMIZED: Use Entry API - only clone if not already cached
        cache.entry(service.id.clone())
            .or_insert_with(|| service.clone());

        if let Some(cap_key) = capability_key {
            let mut cap_cache = self.capability_cache.write().await;
            cap_cache
                .entry(cap_key)
                .or_insert_with(Vec::new)
                .push(service.id.clone());
        }
    }

    /// Remove a service from the cache
    pub async fn remove_from_cache(&self, service_id: &str) {
        let mut cache = self.cache.write().await;
        if let Some(service) = cache.remove(service_id) {
            info!("🗑️  Removed service from cache: {}", service.name);

            // Also remove from capability cache
            let mut cap_cache = self.capability_cache.write().await;
            for service_ids in cap_cache.values_mut() {
                service_ids.retain(|id| id != service_id);
            }
        }
    }

    /// Refresh a cached service
    pub async fn refresh_service(&self, service_id: &str) -> ToadStoolResult<DiscoveredService> {
        // Remove from cache and rediscover
        self.remove_from_cache(service_id).await;

        // Get service capabilities from old cache to help rediscovery
        // For now, just return an error - caller should rediscover by capability
        Err(ToadStoolError::not_found(
            "Service refresh requires capability-based rediscovery",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::service_discovery::DiscoveryMethod;

    #[tokio::test]
    async fn test_discovery_manager_creation() {
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .expect("Failed to create discovery client");
        let manager = DiscoveryManager::new(Arc::new(discovery));

        let cached = manager.get_all_cached().await;
        assert_eq!(cached.len(), 0, "Cache should be empty initially");
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .expect("Failed to create discovery client");
        let manager = DiscoveryManager::new(Arc::new(discovery));

        // Test clear cache
        manager.clear_cache().await;
        let cached = manager.get_all_cached().await;
        assert_eq!(cached.len(), 0);
    }
}
