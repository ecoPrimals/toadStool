// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Discovery engine - orchestrates capability discovery from multiple sources
//!
//! This engine implements the core infant discovery logic where services
//! start with zero knowledge and discover everything dynamically.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::capabilities::{
    CapabilityDiscovery, DetectedSubstrate, DiscoveredService, DiscoveryError,
    DiscoveryPreferences, DiscoverySource, EndpointSource, ServiceHealth, ServiceMetadata,
    SubstrateDetector, SubstrateType,
};

impl From<&str> for DiscoverySource {
    fn from(source_name: &str) -> Self {
        match source_name {
            "environment" => Self::Environment,
            "mdns" => Self::MDNS,
            "service_mesh" => Self::ServiceMesh("unknown".to_string()),
            "config_file" => Self::ConfigFile,
            _ => Self::Fallback,
        }
    }
}

/// Main discovery engine - orchestrates all discovery mechanisms
pub struct DiscoveryEngine {
    /// Registered endpoint sources (in priority order).
    sources: Arc<RwLock<Vec<Box<dyn EndpointSource>>>>,

    /// Registered substrate detectors.
    detectors: Arc<RwLock<Vec<Box<dyn SubstrateDetector>>>>,

    /// Cache of discovered services.
    cache: Arc<RwLock<std::collections::HashMap<String, DiscoveredService>>>,

    /// Configuration.
    config: ServiceDiscoveryConfig,
}

/// Service discovery engine configuration
#[derive(Debug, Clone)]
pub struct ServiceDiscoveryConfig {
    /// Enable caching of discovered services.
    pub enable_cache: bool,

    /// Cache TTL.
    pub cache_ttl: Duration,

    /// Default discovery timeout.
    pub default_timeout: Duration,

    /// Number of retry attempts.
    pub retry_attempts: u32,

    /// Retry delay.
    pub retry_delay: Duration,
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            default_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

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
    pub async fn register_source(&self, source: Box<dyn EndpointSource>) {
        let mut sources = self.sources.write().await;
        sources.push(source);
    }

    /// Register a substrate detector.
    pub async fn register_detector(&self, detector: Box<dyn SubstrateDetector>) {
        let mut detectors = self.detectors.write().await;
        detectors.push(detector);
    }

    /// Discover endpoint by trying each source in order
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError::CapabilityNotFound` if no source can resolve the capability.
    pub async fn discover_endpoint(&self, capability: &str) -> Result<String, DiscoveryError> {
        // Try each source in order
        let sources = self.sources.read().await;

        for source in sources.iter() {
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
        let detectors = self.detectors.read().await;

        for detector in detectors.iter() {
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

    /// Get from cache if available and not expired
    async fn get_from_cache(&self, capability: &str) -> Option<DiscoveredService> {
        if !self.config.enable_cache {
            return None;
        }

        let cache = self.cache.read().await;
        let service = cache.get(capability)?;

        // Check if expired
        let elapsed = service.metadata.last_seen.elapsed().ok()?;
        if elapsed > self.config.cache_ttl {
            return None;
        }

        Some(service.clone())
    }

    /// Store in cache
    async fn store_in_cache(&self, service: DiscoveredService) {
        if !self.config.enable_cache {
            return;
        }

        let mut cache = self.cache.write().await;
        cache.insert(service.capability.clone(), service);
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityDiscovery for DiscoveryEngine {
    fn discover(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            // Check cache first
            if let Some(cached) = self.get_from_cache(&capability_str).await {
                tracing::debug!(capability = capability_str, "Using cached discovery");
                return Ok(cached);
            }

            // Discover endpoint
            let endpoint = self.discover_endpoint(&capability_str).await?;

            // Create discovered service
            let service = DiscoveredService {
                capability: capability_str.clone(),
                endpoint,
                protocols: vec!["http".to_string()], // Default, should be detected
                metadata: ServiceMetadata {
                    version: None,
                    health: ServiceHealth::Unknown,
                    last_seen: std::time::SystemTime::now(),
                    priority: 50,
                    extra: std::collections::HashMap::new(),
                },
                source: DiscoverySource::UniversalAdapter,
            };

            // Cache the result
            self.store_in_cache(service.clone()).await;

            Ok(service)
        })
    }

    fn discover_with_preferences(
        &self,
        capability: &str,
        preferences: DiscoveryPreferences,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            let timeout = preferences.timeout.unwrap_or(self.config.default_timeout);

            // Discover service with timeout
            let discovered =
                match tokio::time::timeout(timeout, self.discover(&capability_str)).await {
                    Ok(result) => result?,
                    Err(_) => return Err(DiscoveryError::Timeout(timeout)),
                };

            // Filter by required protocols if specified
            if !preferences.required_protocols.is_empty() {
                let has_all_protocols = preferences.required_protocols.iter().all(|required| {
                    discovered
                        .protocols
                        .iter()
                        .any(|protocol| protocol == required)
                });

                if !has_all_protocols {
                    return Err(DiscoveryError::ProtocolNotSupported(
                        preferences.required_protocols.join(", "),
                    ));
                }
            }

            // Filter by minimum health level
            if discovered.metadata.health < preferences.min_health {
                return Err(DiscoveryError::NoHealthyServices(capability_str.clone()));
            }

            // Prefer local if requested
            if preferences.prefer_local {
                // Check if endpoint is localhost/127.0.0.1
                let is_local = discovered
                    .endpoint
                    .contains(crate::constants::DEFAULT_HOSTNAME)
                    || discovered
                        .endpoint
                        .contains(crate::constants::LOCALHOST_IPV4)
                    || discovered
                        .endpoint
                        .contains(crate::constants::LOCALHOST_IPV6);

                if !is_local {
                    // Try to find a local alternative from cache
                    let cache = self.cache.read().await;
                    if let Some(cached) = cache.get(&capability_str) {
                        let is_cached_local =
                            cached.endpoint.contains(crate::constants::DEFAULT_HOSTNAME)
                                || cached.endpoint.contains(crate::constants::LOCALHOST_IPV4)
                                || cached.endpoint.contains(crate::constants::LOCALHOST_IPV6);
                        if is_cached_local {
                            return Ok(cached.clone());
                        }
                    }
                }
            }

            Ok(discovered)
        })
    }

    fn discover_all(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<DiscoveredService>, DiscoveryError>>
                + Send
                + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            let mut discovered_services = Vec::new();
            let sources = self.sources.read().await;

            // Query all sources sequentially (avoid lifetime issues)
            for source in sources.iter() {
                match source.resolve(&capability_str).await {
                    Ok(Some(endpoint)) => {
                        // Create discovered service from this endpoint
                        let service = DiscoveredService {
                            capability: capability_str.clone(),
                            endpoint,
                            protocols: vec!["http".to_string()], // Default to HTTP
                            metadata: ServiceMetadata {
                                version: None,
                                health: ServiceHealth::Unknown,
                                last_seen: std::time::SystemTime::now(),
                                priority: 50,
                                extra: std::collections::HashMap::new(),
                            },
                            source: DiscoverySource::from(source.source_name()),
                        };

                        discovered_services.push(service);
                    }
                    Ok(None) => {
                        tracing::trace!(
                            "Source {} returned no results for {}",
                            source.source_name(),
                            capability_str
                        );
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Source {} query failed for {}: {}",
                            source.source_name(),
                            capability_str,
                            e
                        );
                    }
                }
            }

            if discovered_services.is_empty() {
                return Err(DiscoveryError::CapabilityNotFound(capability_str.clone()));
            }

            // Deduplicate by endpoint
            discovered_services.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
            discovered_services.dedup_by(|a, b| a.endpoint == b.endpoint);

            Ok(discovered_services)
        })
    }
}

/// Builder for discovery engine with fluent API
pub struct DiscoveryEngineBuilder {
    config: ServiceDiscoveryConfig,
    sources: Vec<Box<dyn EndpointSource>>,
    detectors: Vec<Box<dyn SubstrateDetector>>,
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
    pub fn with_source(mut self, source: Box<dyn EndpointSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Add a substrate detector
    #[must_use]
    pub fn with_detector(mut self, detector: Box<dyn SubstrateDetector>) -> Self {
        self.detectors.push(detector);
        self
    }

    /// Build the discovery engine
    pub async fn build(self) -> DiscoveryEngine {
        let engine = DiscoveryEngine::with_config(self.config);

        // Register all sources
        for source in self.sources {
            engine.register_source(source).await;
        }

        // Register all detectors
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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSource {
        name: String,
        endpoint: Option<String>,
    }

    impl EndpointSource for MockSource {
        fn resolve(
            &self,
            _service: &str,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<Option<String>, DiscoveryError>>
                    + Send
                    + '_,
            >,
        > {
            let endpoint = self.endpoint.clone();
            Box::pin(async move { Ok(endpoint) })
        }

        fn source_name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_engine_creation() {
        let engine = DiscoveryEngine::new();
        assert!(engine.config.enable_cache);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_with_mock_source() {
        let engine = DiscoveryEngine::new();

        // Register a mock source
        engine
            .register_source(Box::new(MockSource {
                name: "mock".to_string(),
                endpoint: Some("http://localhost:8080".to_string()),
            }))
            .await;

        // Discover endpoint
        let result = engine.discover_endpoint("test_capability").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://localhost:8080");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_fallback() {
        let engine = DiscoveryEngine::new();

        // Register multiple sources (first fails, second succeeds)
        engine
            .register_source(Box::new(MockSource {
                name: "failing".to_string(),
                endpoint: None,
            }))
            .await;

        engine
            .register_source(Box::new(MockSource {
                name: "working".to_string(),
                endpoint: Some("http://fallback:9090".to_string()),
            }))
            .await;

        // Should get the fallback endpoint
        let result = engine.discover_endpoint("test_capability").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://fallback:9090");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_builder_pattern() {
        let engine = DiscoveryEngineBuilder::new()
            .timeout(Duration::from_secs(10))
            .cache_ttl(Duration::from_secs(60))
            .with_source(Box::new(MockSource {
                name: "test".to_string(),
                endpoint: Some("http://test:8080".to_string()),
            }))
            .build()
            .await;

        assert_eq!(engine.config.default_timeout, Duration::from_secs(10));
        assert_eq!(engine.config.cache_ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_discovery_config_default() {
        let config = ServiceDiscoveryConfig::default();

        assert!(config.enable_cache);
        assert_eq!(config.cache_ttl, Duration::from_secs(300));
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_engine_default() {
        let engine = DiscoveryEngine::default();
        assert!(engine.config.enable_cache);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_engine_with_config() {
        let config = ServiceDiscoveryConfig {
            enable_cache: false,
            cache_ttl: Duration::from_secs(600),
            default_timeout: Duration::from_secs(60),
            retry_attempts: 5,
            retry_delay: Duration::from_secs(2),
        };

        let engine = DiscoveryEngine::with_config(config.clone());

        assert!(!engine.config.enable_cache);
        assert_eq!(engine.config.cache_ttl, Duration::from_secs(600));
        assert_eq!(engine.config.default_timeout, Duration::from_secs(60));
        assert_eq!(engine.config.retry_attempts, 5);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_clear_cache() {
        let engine = DiscoveryEngine::new();

        // Clear cache (should not panic even when empty)
        engine.clear_cache().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_substrate_no_detectors() {
        let engine = DiscoveryEngine::new();

        // With no detectors, should return bare metal
        let substrate = engine
            .detect_substrate()
            .await
            .expect("Should detect substrate");
        assert_eq!(substrate.substrate_type, super::SubstrateType::Bare);
        assert!(substrate.capabilities.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discovery_endpoint_not_found() {
        let engine = DiscoveryEngine::new();

        // With no sources, should fail
        let result = engine.discover_endpoint("missing_capability").await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_builder_disable_cache() {
        let engine = DiscoveryEngineBuilder::new().disable_cache().build().await;

        assert!(!engine.config.enable_cache);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_builder_default() {
        let builder = DiscoveryEngineBuilder::default();
        let engine = builder.build().await;

        assert!(engine.config.enable_cache);
    }

    struct MockDetector {
        name: String,
        result: Option<super::DetectedSubstrate>,
    }

    impl super::SubstrateDetector for MockDetector {
        fn detect(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<Option<super::DetectedSubstrate>, super::DiscoveryError>,
                    > + Send
                    + '_,
            >,
        > {
            let result = self.result.clone();
            Box::pin(async move { Ok(result) })
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_detect_substrate_with_detector() {
        let engine = DiscoveryEngine::new();

        let substrate = super::DetectedSubstrate {
            substrate_type: super::SubstrateType::ContainerOrchestrator,
            capabilities: vec![
                crate::infant_discovery::SubstrateCapability::ContainerOrchestration,
            ],
            metadata: std::collections::HashMap::new(),
        };

        engine
            .register_detector(Box::new(MockDetector {
                name: "test_detector".to_string(),
                result: Some(substrate.clone()),
            }))
            .await;

        let detected = engine.detect_substrate().await.expect("Should detect");
        assert_eq!(
            detected.substrate_type,
            super::SubstrateType::ContainerOrchestrator
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_discovery_trait() {
        use super::CapabilityDiscovery;

        let engine = DiscoveryEngine::new();

        // Register a mock source
        engine
            .register_source(Box::new(MockSource {
                name: "test".to_string(),
                endpoint: Some("http://localhost:9999".to_string()),
            }))
            .await;

        // Test discover
        let service = engine
            .discover("test_capability")
            .await
            .expect("Should discover");
        assert_eq!(service.capability, "test_capability");
        assert_eq!(service.endpoint, "http://localhost:9999");

        // Test is_available (should use cached result)
        let available = engine.is_available("test_capability").await;
        assert!(available);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discover_with_preferences() {
        use super::CapabilityDiscovery;

        let engine = DiscoveryEngine::new();

        engine
            .register_source(Box::new(MockSource {
                name: "test".to_string(),
                endpoint: Some("http://localhost:7777".to_string()),
            }))
            .await;

        let prefs = super::DiscoveryPreferences {
            prefer_local: true,
            required_protocols: vec![],
            timeout: Some(Duration::from_secs(10)),
            min_health: super::ServiceHealth::Unknown,
            preferred_sources: vec![],
        };

        let service = engine
            .discover_with_preferences("test", prefs)
            .await
            .expect("Should discover");
        assert_eq!(service.endpoint, "http://localhost:7777");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discover_all() {
        use super::CapabilityDiscovery;

        let engine = DiscoveryEngine::new();

        engine
            .register_source(Box::new(MockSource {
                name: "test".to_string(),
                endpoint: Some("http://localhost:6666".to_string()),
            }))
            .await;

        let services = engine.discover_all("test").await.expect("Should discover");
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].endpoint, "http://localhost:6666");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_builder_fluent_api() {
        let engine = DiscoveryEngineBuilder::new()
            .cache_ttl(Duration::from_secs(120))
            .timeout(Duration::from_secs(15))
            .disable_cache()
            .with_source(Box::new(MockSource {
                name: "source1".to_string(),
                endpoint: Some("http://s1:8080".to_string()),
            }))
            .with_source(Box::new(MockSource {
                name: "source2".to_string(),
                endpoint: Some("http://s2:8080".to_string()),
            }))
            .build()
            .await;

        assert_eq!(engine.config.cache_ttl, Duration::from_secs(120));
        assert_eq!(engine.config.default_timeout, Duration::from_secs(15));
        assert!(!engine.config.enable_cache);
    }
}
