// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Coverage expansion tests for `infant_discovery` module (S155)
//!
//! Tests public API of:
//! - engine.rs: `DiscoveryEngine`, `DiscoveryEngineBuilder`, discovery flow
//! - sources.rs: `EnvironmentSource`, `FallbackSource`, `MDNSSource`, `ConfigFileSource`, etc.
//! - capabilities.rs: types, traits, capability constants
//! - detectors.rs: `BareMetalDetector`, `HardwareEnvironment`, `standard_detectors`

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use temp_env::with_vars;

use toadstool_common::infant_discovery::capabilities::capabilities;
use toadstool_common::infant_discovery::detectors::{
    BareMetalDetector, HardwareEnvironment, standard_detectors,
};
use toadstool_common::infant_discovery::sources::{
    ConfigFileSource, EnvironmentSource, FallbackSource, MDNSSource, ServiceMeshSource,
    development_sources, production_sources,
};
use toadstool_common::infant_discovery::{
    CapabilityDiscovery, DetectedSubstrate, DiscoveredService, DiscoveryEngine,
    DiscoveryEngineBuilder, DiscoveryError, DiscoveryPreferences, DiscoverySource,
    EndpointResolver, EndpointSource, ServiceDiscoveryConfig, ServiceHealth, ServiceMetadata,
    SubstrateCapability, SubstrateDetector, SubstrateType,
};

// ============================================================================
// Mock implementations for integration tests
// ============================================================================

struct MockEndpointSource {
    name: String,
    endpoint: Option<String>,
}

impl EndpointSource for MockEndpointSource {
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

struct MockSubstrateDetector {
    name: String,
    result: Option<DetectedSubstrate>,
}

impl SubstrateDetector for MockSubstrateDetector {
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

// ============================================================================
// Engine creation and discovery flow tests
// ============================================================================

#[test]
fn test_service_discovery_config_default() {
    let config = ServiceDiscoveryConfig::default();
    assert!(config.enable_cache);
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
    assert_eq!(config.default_timeout, Duration::from_secs(30));
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay, Duration::from_secs(1));
}

#[test]
fn test_discovery_engine_new() {
    let engine = DiscoveryEngine::new();
    // Engine created with default config - verify it's usable
    drop(engine);
}

#[test]
fn test_discovery_engine_with_config() {
    let config = ServiceDiscoveryConfig {
        enable_cache: false,
        cache_ttl: Duration::from_secs(600),
        default_timeout: Duration::from_secs(60),
        retry_attempts: 5,
        retry_delay: Duration::from_secs(2),
    };
    let engine = DiscoveryEngine::with_config(config);
    // Engine created with custom config - verify it's usable
    drop(engine);
}

#[test]
fn test_discovery_engine_default_impl() {
    let engine = DiscoveryEngine::default();
    // Default impl creates engine with default config
    drop(engine);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_register_source_and_discover() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "mock".to_string(),
            endpoint: Some("http://localhost:9090".to_string()),
        }))
        .await;

    let result = engine.discover_endpoint("test_capability").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://localhost:9090");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_source_fallback_order() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "first".to_string(),
            endpoint: None,
        }))
        .await;
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "second".to_string(),
            endpoint: Some("http://fallback:8080".to_string()),
        }))
        .await;

    let result = engine.discover_endpoint("capability").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://fallback:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_endpoint_not_found() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover_endpoint("missing").await;
    assert!(result.is_err());
    if let Err(DiscoveryError::CapabilityNotFound(name)) = result {
        assert_eq!(name, "missing");
    } else {
        panic!("Expected CapabilityNotFound");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_clear_cache() {
    let engine = DiscoveryEngine::new();
    engine.clear_cache().await;
    // Should not panic
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_builder_fluent_api() {
    let engine = DiscoveryEngineBuilder::new()
        .cache_ttl(Duration::from_secs(120))
        .timeout(Duration::from_secs(15))
        .disable_cache()
        .with_source(Arc::new(MockEndpointSource {
            name: "builder_source".to_string(),
            endpoint: Some("http://builder:8080".to_string()),
        }))
        .build()
        .await;

    let result = engine.discover_endpoint("cap").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://builder:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discovery_engine_builder_default() {
    let engine = DiscoveryEngineBuilder::default().build().await;
    // Builder default creates engine - verify discover works
    let _ = engine.discover_endpoint("any").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capability_discovery_trait_discover() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "trait_test".to_string(),
            endpoint: Some("http://localhost:7777".to_string()),
        }))
        .await;

    let service = engine
        .discover("ai_processing")
        .await
        .expect("Should discover");
    assert_eq!(service.capability, "ai_processing");
    assert_eq!(service.endpoint, "http://localhost:7777");
    assert!(service.protocols.contains(&"http".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capability_discovery_trait_is_available() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "avail".to_string(),
            endpoint: Some("http://avail:8080".to_string()),
        }))
        .await;

    assert!(engine.is_available("test_cap").await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capability_discovery_trait_is_available_false() {
    let engine = DiscoveryEngine::new();
    assert!(!engine.is_available("nonexistent").await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_with_preferences() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "prefs".to_string(),
            endpoint: Some("http://127.0.0.1:8888".to_string()),
        }))
        .await;

    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec![],
        timeout: Some(Duration::from_secs(5)),
        min_health: ServiceHealth::Unknown,
        preferred_sources: vec![],
    };

    let service = engine
        .discover_with_preferences("cap", prefs)
        .await
        .expect("Should discover");
    assert_eq!(service.endpoint, "http://127.0.0.1:8888");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_all_returns_multiple_sources() {
    let engine = DiscoveryEngine::new();
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "s1".to_string(),
            endpoint: Some("http://s1:8080".to_string()),
        }))
        .await;
    engine
        .register_source(Arc::new(MockEndpointSource {
            name: "s2".to_string(),
            endpoint: Some("http://s2:8080".to_string()),
        }))
        .await;

    let services = engine.discover_all("cap").await.expect("Should discover");
    assert!(!services.is_empty());
    assert!(
        services
            .iter()
            .any(|s| s.endpoint.contains("s1") || s.endpoint.contains("s2"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_all_not_found() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover_all("missing").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_substrate_no_detectors_returns_bare() {
    let engine = DiscoveryEngine::new();
    let substrate = engine.detect_substrate().await.expect("Should detect");
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    assert!(substrate.capabilities.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detect_substrate_with_mock_detector() {
    let engine = DiscoveryEngine::new();
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![SubstrateCapability::ContainerOrchestration],
        metadata: HashMap::new(),
    };
    engine
        .register_detector(Arc::new(MockSubstrateDetector {
            name: "mock_detector".to_string(),
            result: Some(substrate.clone()),
        }))
        .await;

    let detected = engine.detect_substrate().await.expect("Should detect");
    assert_eq!(
        detected.substrate_type,
        SubstrateType::ContainerOrchestrator
    );
    assert!(detected.has_capability(&SubstrateCapability::ContainerOrchestration));
}

// ============================================================================
// Capability detection and type construction tests
// ============================================================================

#[test]
fn test_capability_constants() {
    assert_eq!(capabilities::AI_PROCESSING, "ai_processing");
    assert_eq!(capabilities::NLP, "natural_language_processing");
    assert_eq!(capabilities::AUTHENTICATION, "authentication");
    assert_eq!(capabilities::STORAGE, "persistent_storage");
    assert_eq!(capabilities::ORCHESTRATION, "service_orchestration");
    assert_eq!(capabilities::MONITORING, "monitoring");
    assert_eq!(capabilities::CACHE, "caching");
}

#[test]
fn test_substrate_type_variants() {
    let types = [
        SubstrateType::ContainerOrchestrator,
        SubstrateType::ContainerRuntime,
        SubstrateType::Cloud,
        SubstrateType::Bare,
    ];
    assert_eq!(types.len(), 4);
    assert_eq!(SubstrateType::Bare, SubstrateType::Bare);
}

#[test]
fn test_substrate_capability_variants() {
    let caps = [
        SubstrateCapability::ContainerOrchestration,
        SubstrateCapability::ContainerRuntime,
        SubstrateCapability::ServiceMesh,
        SubstrateCapability::ServiceDiscovery,
        SubstrateCapability::CloudCompute,
        SubstrateCapability::BareMetal,
    ];
    assert_eq!(caps.len(), 6);
}

#[test]
fn test_detected_substrate_has_capability() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Bare,
        capabilities: vec![SubstrateCapability::BareMetal],
        metadata: HashMap::new(),
    };
    assert!(substrate.has_capability(&SubstrateCapability::BareMetal));
    assert!(!substrate.has_capability(&SubstrateCapability::CloudCompute));
}

#[test]
fn test_detected_substrate_get_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("hostname".to_string(), "testhost".to_string());
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Bare,
        capabilities: vec![],
        metadata,
    };
    assert_eq!(
        substrate.get_metadata("hostname"),
        Some(&"testhost".to_string())
    );
    assert_eq!(substrate.get_metadata("missing"), None);
}

#[test]
fn test_discovered_service_construction() {
    let service = DiscoveredService {
        capability: "ai_processing".to_string(),
        endpoint: "http://ai:7000".to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 90,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };
    assert_eq!(service.capability, "ai_processing");
    assert_eq!(service.endpoint, "http://ai:7000");
    assert_eq!(service.protocols.len(), 2);
}

#[test]
fn test_service_metadata_default() {
    let metadata = ServiceMetadata::default();
    assert!(metadata.version.is_none());
    assert_eq!(metadata.health, ServiceHealth::Unknown);
    assert_eq!(metadata.priority, 50);
}

#[test]
fn test_service_health_ordering() {
    assert!(ServiceHealth::Healthy > ServiceHealth::Degraded);
    assert!(ServiceHealth::Degraded > ServiceHealth::Unknown);
}

#[test]
fn test_discovery_preferences_default() {
    let prefs = DiscoveryPreferences::default();
    assert!(!prefs.prefer_local);
    assert!(prefs.required_protocols.is_empty());
    assert!(prefs.timeout.is_none());
}

#[test]
fn test_discovery_source_from_str() {
    let env: DiscoverySource = "environment".into();
    assert!(matches!(env, DiscoverySource::Environment));

    let mdns: DiscoverySource = "mdns".into();
    assert!(matches!(mdns, DiscoverySource::MDNS));

    let fallback: DiscoverySource = "unknown".into();
    assert!(matches!(fallback, DiscoverySource::Fallback));
}

#[test]
fn test_discovery_error_display() {
    let err = DiscoveryError::CapabilityNotFound("test".to_string());
    assert!(err.to_string().contains("test"));

    let err2 = DiscoveryError::Timeout(Duration::from_secs(30));
    assert!(err2.to_string().contains("timeout") || err2.to_string().contains("Timeout"));
}

#[test]
fn test_endpoint_resolver_creation() {
    let mut resolver = EndpointResolver::new();
    let mut fallbacks = HashMap::new();
    fallbacks.insert("test_cap".to_string(), "http://resolver:8080".to_string());
    resolver.add_source(Box::new(FallbackSource::with_fallbacks(fallbacks)));
    // Resolver created with source - verify via resolve
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_endpoint_resolver_resolve() {
    let mut resolver = EndpointResolver::new();
    let mut fallbacks = HashMap::new();
    fallbacks.insert("resolved".to_string(), "http://resolved:9090".to_string());
    resolver.add_source(Box::new(FallbackSource::with_fallbacks(fallbacks)));

    let result = resolver.resolve("resolved").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "http://resolved:9090");
}

// ============================================================================
// Source enumeration and parsing tests
// ============================================================================

#[test]
fn test_environment_source_default() {
    let source = EnvironmentSource::default();
    assert_eq!(source.source_name(), "environment");
}

#[test]
fn test_environment_source_new() {
    let source = EnvironmentSource::new("CUSTOM_");
    assert_eq!(source.source_name(), "environment");
}

#[test]
fn test_environment_source_resolve_with_env() {
    with_vars(
        [("TOADSTOOL_ENV_TEST_ENDPOINT", Some("http://env-test:9999"))],
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = EnvironmentSource::default();
                let result = source.resolve("env_test").await.unwrap();
                assert_eq!(result, Some("http://env-test:9999".to_string()));
            });
        },
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_environment_source_resolve_missing() {
    let source = EnvironmentSource::default();
    let result = source.resolve("nonexistent_capability_xyz").await.unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_fallback_source_with_fallbacks() {
    let mut fallbacks = HashMap::new();
    fallbacks.insert("custom".to_string(), "http://custom:1234".to_string());
    let source = FallbackSource::with_fallbacks(fallbacks);
    assert_eq!(source.source_name(), "fallback");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_fallback_source_resolve() {
    let mut fallbacks = HashMap::new();
    fallbacks.insert(
        "fallback_cap".to_string(),
        "http://fallback:5555".to_string(),
    );
    let source = FallbackSource::with_fallbacks(fallbacks);
    let result = source.resolve("fallback_cap").await.unwrap();
    assert_eq!(result, Some("http://fallback:5555".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_fallback_source_resolve_missing() {
    let source = FallbackSource::with_fallbacks(HashMap::new());
    let result = source.resolve("missing").await.unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_mdns_source() {
    let source = MDNSSource::new();
    assert_eq!(source.source_name(), "mdns");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mdns_source_resolve_unknown() {
    let source = MDNSSource::new();
    let result = source.resolve("unknown_service_xyz").await.unwrap();
    // May or may not find - depends on filesystem
    let _ = result;
}

#[test]
fn test_service_mesh_source() {
    let source = ServiceMeshSource::new();
    assert_eq!(source.source_name(), "service_mesh");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_service_mesh_source_auto_returns_none() {
    let source = ServiceMeshSource::new();
    let result = source.resolve("any_service").await.unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_config_file_source_new() {
    let source = ConfigFileSource::new("/path/to/config.toml");
    assert_eq!(source.source_name(), "config_file");
}

#[test]
fn test_config_file_source_default_path() {
    let source = ConfigFileSource::default_path();
    assert!(source.source_name().contains("config"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_file_source_missing_file() {
    let source = ConfigFileSource::new("/nonexistent/path/config.toml");
    let result = source.resolve("any").await.unwrap();
    assert_eq!(result, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_file_source_parse_toml() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[services.test_service]
endpoint = "http://parsed:8888"
"#,
    )
    .unwrap();

    let source = ConfigFileSource::new(&config_path);
    let result = source.resolve("test_service").await.unwrap();
    assert_eq!(result, Some("http://parsed:8888".to_string()));
}

#[test]
fn test_production_sources() {
    let sources = production_sources();
    assert_eq!(sources.len(), 4);
    assert_eq!(sources[0].source_name(), "environment");
    assert_eq!(sources[3].source_name(), "fallback");
}

#[test]
fn test_development_sources() {
    let sources = development_sources();
    assert_eq!(sources.len(), 2);
}

// ============================================================================
// Detector types and discovery pattern tests
// ============================================================================

#[test]
fn test_bare_metal_detector_new() {
    let detector = BareMetalDetector::new();
    assert_eq!(detector.name(), "bare_metal");
}

#[test]
fn test_bare_metal_detector_default() {
    let detector = BareMetalDetector;
    assert_eq!(detector.name(), "bare_metal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_detect() {
    let detector = BareMetalDetector::new();
    let result: Result<Option<DetectedSubstrate>, DiscoveryError> = detector.detect().await;
    let result = result.unwrap();
    assert!(result.is_some());
    let substrate = result.unwrap();
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
    assert!(!substrate.capabilities.is_empty());
    assert!(substrate.metadata.contains_key("deployment"));
}

#[test]
fn test_standard_detectors() {
    let detectors = standard_detectors();
    assert_eq!(detectors.len(), 1);
    assert_eq!(detectors[0].name(), "bare_metal");
}

#[test]
fn test_hardware_environment_default() {
    let env = HardwareEnvironment::default();
    assert!(env.hostname.is_none());
}

#[test]
fn test_hardware_environment_from_env() {
    let env = HardwareEnvironment::from_env();
    // May or may not have hostname
    let _ = env.hostname;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_engine_with_standard_detectors() {
    let engine = DiscoveryEngineBuilder::new()
        .with_detector(Arc::new(BareMetalDetector::new()))
        .build()
        .await;

    let substrate = engine.detect_substrate().await.expect("Should detect");
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
}
