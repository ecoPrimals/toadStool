// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use super::DiscoveryEngine;
use crate::infant_discovery::DiscoveryEngineBuilder;
use crate::infant_discovery::SubstrateCapability;
use crate::infant_discovery::capabilities::{
    CapabilityDiscovery, DetectedSubstrate, DiscoveryError, DiscoveryPreferences, EndpointSource,
    ServiceHealth, SubstrateDetector, SubstrateType,
};
use crate::infant_discovery::config::ServiceDiscoveryConfig;
use std::sync::Arc;
use std::time::Duration;

struct MockSource {
    name: String,
    endpoint: Option<String>,
}

impl EndpointSource for MockSource {
    fn resolve(
        &self,
        _service: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>,
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

    engine
        .register_source(Arc::new(MockSource {
            name: "mock".to_string(),
            endpoint: Some("http://localhost:8080".to_string()),
        }))
        .await;

    let result = engine.discover_endpoint("test_capability").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://localhost:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_fallback() {
    let engine = DiscoveryEngine::new();

    engine
        .register_source(Arc::new(MockSource {
            name: "failing".to_string(),
            endpoint: None,
        }))
        .await;

    engine
        .register_source(Arc::new(MockSource {
            name: "working".to_string(),
            endpoint: Some("http://fallback:9090".to_string()),
        }))
        .await;

    let result = engine.discover_endpoint("test_capability").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://fallback:9090");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_builder_pattern() {
    let engine = DiscoveryEngineBuilder::new()
        .timeout(Duration::from_secs(10))
        .cache_ttl(Duration::from_secs(60))
        .with_source(Arc::new(MockSource {
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

    let engine = DiscoveryEngine::with_config(config);

    assert!(!engine.config.enable_cache);
    assert_eq!(engine.config.cache_ttl, Duration::from_secs(600));
    assert_eq!(engine.config.default_timeout, Duration::from_secs(60));
    assert_eq!(engine.config.retry_attempts, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_clear_cache() {
    let engine = DiscoveryEngine::new();
    engine.clear_cache().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_substrate_no_detectors() {
    let engine = DiscoveryEngine::new();

    let substrate = engine
        .detect_substrate()
        .await
        .expect("Should detect substrate");
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    assert!(substrate.capabilities.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_endpoint_not_found() {
    let engine = DiscoveryEngine::new();
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
    result: Option<DetectedSubstrate>,
}

impl SubstrateDetector for MockDetector {
    fn detect(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>>
                + Send
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

    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![SubstrateCapability::ContainerOrchestration],
        metadata: std::collections::HashMap::new(),
    };

    engine
        .register_detector(Arc::new(MockDetector {
            name: "test_detector".to_string(),
            result: Some(substrate.clone()),
        }))
        .await;

    let detected = engine.detect_substrate().await.expect("Should detect");
    assert_eq!(
        detected.substrate_type,
        SubstrateType::ContainerOrchestrator
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capability_discovery_trait() {
    let engine = DiscoveryEngine::new();

    engine
        .register_source(Arc::new(MockSource {
            name: "test".to_string(),
            endpoint: Some("http://localhost:9999".to_string()),
        }))
        .await;

    let service = engine
        .discover("test_capability")
        .await
        .expect("Should discover");
    assert_eq!(service.capability, "test_capability");
    assert_eq!(service.endpoint, "http://localhost:9999");

    let available = engine.is_available("test_capability").await;
    assert!(available);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_with_preferences() {
    let engine = DiscoveryEngine::new();

    engine
        .register_source(Arc::new(MockSource {
            name: "test".to_string(),
            endpoint: Some("http://localhost:7777".to_string()),
        }))
        .await;

    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec![],
        timeout: Some(Duration::from_secs(10)),
        min_health: ServiceHealth::Unknown,
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
    let engine = DiscoveryEngine::new();

    engine
        .register_source(Arc::new(MockSource {
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
        .with_source(Arc::new(MockSource {
            name: "source1".to_string(),
            endpoint: Some("http://s1:8080".to_string()),
        }))
        .with_source(Arc::new(MockSource {
            name: "source2".to_string(),
            endpoint: Some("http://s2:8080".to_string()),
        }))
        .build()
        .await;

    assert_eq!(engine.config.cache_ttl, Duration::from_secs(120));
    assert_eq!(engine.config.default_timeout, Duration::from_secs(15));
    assert!(!engine.config.enable_cache);
}
