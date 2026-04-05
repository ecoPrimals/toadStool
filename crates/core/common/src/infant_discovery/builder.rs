// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fluent builder for [`DiscoveryEngine`]

use std::sync::Arc;
use std::time::Duration;

use super::capabilities::{EndpointSource, SubstrateDetector};
use super::config::ServiceDiscoveryConfig;
use super::engine::DiscoveryEngine;

/// Builder for discovery engine with fluent API
pub struct DiscoveryEngineBuilder {
    config: ServiceDiscoveryConfig,
    sources: Vec<Arc<dyn EndpointSource>>,
    detectors: Vec<Arc<dyn SubstrateDetector>>,
}

impl DiscoveryEngineBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ServiceDiscoveryConfig::default(),
            sources: Vec::new(),
            detectors: Vec::new(),
        }
    }

    /// Set cache TTL
    #[must_use]
    pub const fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.config.cache_ttl = ttl;
        self
    }

    /// Set default timeout
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    /// Disable caching
    #[must_use]
    pub const fn disable_cache(mut self) -> Self {
        self.config.enable_cache = false;
        self
    }

    /// Add an endpoint source
    #[must_use]
    pub fn with_source(mut self, source: Arc<dyn EndpointSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Add a substrate detector
    #[must_use]
    pub fn with_detector(mut self, detector: Arc<dyn SubstrateDetector>) -> Self {
        self.detectors.push(detector);
        self
    }

    /// Build the discovery engine
    pub async fn build(self) -> DiscoveryEngine {
        let engine = DiscoveryEngine::with_config(self.config);

        for source in self.sources {
            engine.register_source(source).await;
        }

        for detector in self.detectors {
            engine.register_detector(detector).await;
        }

        engine
    }
}

impl Default for DiscoveryEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
