// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use crate::primal_identity::{Capability, DiscoveredService};

/// Service cache for discovered services
///
/// Uses `Arc<DiscoveredService>` internally for zero-copy sharing.
/// This avoids expensive clones during cache operations (insert, lookup).
/// Cloning only happens at API boundaries when returning to callers.
#[derive(Debug)]
pub(super) struct ServiceCache {
    /// Services indexed by capability (Arc for cheap sharing)
    by_capability: HashMap<Capability, Vec<Arc<DiscoveredService>>>,

    /// All services (Arc for cheap sharing)
    all_services: Vec<Arc<DiscoveredService>>,

    /// Cache timestamp
    last_updated: std::time::Instant,
}

impl ServiceCache {
    pub(super) fn new() -> Self {
        Self {
            by_capability: HashMap::new(),
            all_services: Vec::new(),
            last_updated: std::time::Instant::now(),
        }
    }

    pub(super) fn insert(&mut self, service: DiscoveredService) {
        // Wrap in Arc once for zero-copy sharing across indexes
        let service_arc = Arc::new(service);

        // Add to all services (cheap Arc clone - just ref count increment)
        if !self.all_services.iter().any(|s| s.id == service_arc.id) {
            self.all_services.push(Arc::clone(&service_arc));
        }

        // Index by capabilities (cheap Arc clone - just ref count increment)
        for capability in &service_arc.capabilities {
            self.by_capability
                .entry(capability.clone())
                .or_default()
                .push(Arc::clone(&service_arc));
        }

        self.last_updated = std::time::Instant::now();
    }

    pub(super) fn get_by_capability(
        &self,
        capability: &Capability,
    ) -> Option<Vec<Arc<DiscoveredService>>> {
        // Cheap Arc clones (just ref count increments, not deep copies)
        self.by_capability.get(capability).cloned()
    }

    pub(super) fn get_all(&self) -> Vec<Arc<DiscoveredService>> {
        // Cheap Arc clones (just ref count increments, not deep copies)
        self.all_services.clone()
    }

    pub(super) fn clear(&mut self) {
        self.by_capability.clear();
        self.all_services.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::primal_identity::{
        Capability, ComputeCapability, DiscoveredService, ServiceEndpoint,
    };

    use super::ServiceCache;

    #[tokio::test]
    async fn test_service_cache_new() {
        let cache = ServiceCache::new();
        assert!(cache.get_all().is_empty());
    }

    #[tokio::test]
    async fn test_service_cache_insert() {
        let mut cache = ServiceCache::new();

        let service = DiscoveredService {
            id: Some("test-1".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8081)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service);
        assert_eq!(cache.get_all().len(), 1);
    }

    #[tokio::test]
    async fn test_service_cache_get_by_capability() {
        let mut cache = ServiceCache::new();

        let cap = Capability::Compute(ComputeCapability::NativeExecution);

        let service = DiscoveredService {
            id: Some("test-2".to_string()),
            capabilities: vec![cap.clone()],
            endpoints: vec![ServiceEndpoint::http("localhost", 8082)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service);

        let services = cache.get_by_capability(&cap);
        assert!(services.is_some());
        assert_eq!(services.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_service_cache_get_by_capability_none() {
        let cache = ServiceCache::new();

        let cap = Capability::Storage(crate::primal_identity::StorageCapability::ObjectStorage);

        let services = cache.get_by_capability(&cap);
        assert!(services.is_none());
    }

    #[tokio::test]
    async fn test_service_cache_clear() {
        let mut cache = ServiceCache::new();

        let service = DiscoveredService {
            id: Some("test-3".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8083)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service);
        assert!(!cache.get_all().is_empty());

        cache.clear();
        assert!(cache.get_all().is_empty());
    }
}
