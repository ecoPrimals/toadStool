// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Construction, registration, endpoint resolution, substrate detection, and cache clearing.

use std::sync::Arc;
use std::sync::RwLock;

use super::super::capabilities::{
    DetectedSubstrate, DiscoveryError, EndpointSource, SubstrateDetector, SubstrateType,
};
use super::super::config::ServiceDiscoveryConfig;
use super::DiscoveryEngine;

impl DiscoveryEngine {
    /// Create a new discovery engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ServiceDiscoveryConfig::default())
    }

    /// Create a new discovery engine with custom configuration.
    #[must_use]
    pub fn with_config(config: ServiceDiscoveryConfig) -> Self {
        Self {
            sources: Arc::new(RwLock::new(Vec::new())),
            detectors: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            config,
        }
    }

    /// Register an endpoint source.
    pub async fn register_source(&self, source: Arc<dyn EndpointSource>) {
        let mut sources = self.sources.write().unwrap_or_else(|e| e.into_inner());
        sources.push(source);
    }

    /// Register a substrate detector.
    pub async fn register_detector(&self, detector: Arc<dyn SubstrateDetector>) {
        let mut detectors = self.detectors.write().unwrap_or_else(|e| e.into_inner());
        detectors.push(detector);
    }

    /// Discover endpoint by trying each source in order
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError::CapabilityNotFound` if no source can resolve the capability.
    pub async fn discover_endpoint(&self, capability: &str) -> Result<String, DiscoveryError> {
        // Try each source in order (clone to avoid holding lock across await)
        let sources: Vec<Arc<dyn EndpointSource>> = self
            .sources
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(Arc::clone)
            .collect();

        for source in &sources {
            match source.resolve(capability).await {
                Ok(Some(endpoint)) => {
                    tracing::info!(
                        capability = capability,
                        source = source.source_name(),
                        endpoint = endpoint,
                        "Discovered service endpoint"
                    );
                    return Ok(endpoint);
                }
                Ok(None) => {
                    tracing::debug!(
                        capability = capability,
                        source = source.source_name(),
                        "Source did not find endpoint"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        capability = capability,
                        source = source.source_name(),
                        error = ?e,
                        "Source failed to resolve endpoint"
                    );
                }
            }
        }

        Err(DiscoveryError::CapabilityNotFound(capability.to_string()))
    }

    /// Detect the runtime substrate
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if substrate detection fails or no detectors are available.
    pub async fn detect_substrate(&self) -> Result<DetectedSubstrate, DiscoveryError> {
        let detectors: Vec<Arc<dyn SubstrateDetector>> = self
            .detectors
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(Arc::clone)
            .collect();

        for detector in &detectors {
            match detector.detect().await {
                Ok(Some(substrate)) => {
                    tracing::info!(
                        detector = detector.name(),
                        substrate_type = ?substrate.substrate_type,
                        "Detected substrate"
                    );
                    return Ok(substrate);
                }
                Ok(None) => {
                    tracing::debug!(
                        detector = detector.name(),
                        "Detector did not find substrate"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        detector = detector.name(),
                        error = ?e,
                        "Detector failed"
                    );
                }
            }
        }

        // Default to bare metal if nothing detected
        Ok(DetectedSubstrate {
            substrate_type: SubstrateType::Bare,
            capabilities: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap_or_else(|e| e.into_inner());
        cache.clear();
    }
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}
