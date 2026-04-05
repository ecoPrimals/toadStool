// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Discovery engine - orchestrates capability discovery from multiple sources
//!
//! This engine implements the core infant discovery logic where services
//! start with zero knowledge and discover everything dynamically.

use std::sync::Arc;
use tokio::sync::RwLock;

use super::capabilities::{DiscoveredService, EndpointSource, SubstrateDetector};
use super::config::ServiceDiscoveryConfig;

/// Main discovery engine - orchestrates all discovery mechanisms
pub struct DiscoveryEngine {
    /// Registered endpoint sources (in priority order).
    sources: Arc<RwLock<Vec<Arc<dyn EndpointSource>>>>,

    /// Registered substrate detectors.
    detectors: Arc<RwLock<Vec<Arc<dyn SubstrateDetector>>>>,

    /// Cache of discovered services.
    cache: Arc<RwLock<std::collections::HashMap<String, DiscoveredService>>>,

    /// Configuration.
    config: ServiceDiscoveryConfig,
}

impl DiscoveryEngine {
    /// Get from cache if available and not expired
    async fn get_from_cache(&self, capability: &str) -> Option<DiscoveredService> {
        if !self.config.enable_cache {
            return None;
        }

        let service = {
            let cache = self.cache.read().await;
            cache.get(capability)?.clone()
        };

        // Check if expired
        let elapsed = service.metadata.last_seen.elapsed().ok()?;
        if elapsed > self.config.cache_ttl {
            return None;
        }

        Some(service)
    }

    /// Store in cache
    async fn store_in_cache(&self, service: DiscoveredService) {
        if !self.config.enable_cache {
            return;
        }

        let mut cache = self.cache.write().await;
        cache.insert(service.capability.clone(), service);
    }
}

mod capability_discovery;
mod conversions;
mod core;

#[cfg(test)]
mod tests;
