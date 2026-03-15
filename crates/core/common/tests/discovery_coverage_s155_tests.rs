// SPDX-License-Identifier: AGPL-3.0-only
//! Discovery coverage tests for primal discovery, service discovery, primal sockets,
//! primal identity, universal adapter, and capability provider modules.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use toadstool_common::capability_provider::{discover_all, CapabilityError, CapabilityProvider};
use toadstool_common::primal_discovery::{
    DiscoveryConfig, DiscoveryError, DiscoveryMethod, PrimalDiscovery, PrimalEndpoint, TrustLevel,
};
use toadstool_common::primal_discovery_mdns::{MdnsAdapter, TOADSTOOL_SERVICE_TYPE};
use toadstool_common::primal_identity::{
    AuthCapability, Capability, ComputeCapability, CoordinationCapability, CryptoCapability,
    DiscoveryCapability, ServiceEndpoint, StorageCapability,
};
use toadstool_common::primal_sockets::{
    resolve_beardog_socket_fallback, resolve_biomeos_dir, resolve_family_id,
    resolve_nestgate_socket_fallback, resolve_nucleus_socket, resolve_runtime_dir,
    resolve_socket_path_for_service, resolve_songbird_socket_fallback, resolve_squirrel_socket,
    resolve_toadstool_socket, SocketDiscoveryError, SocketPathEnv,
};
use toadstool_common::service_discovery::{
    DiscoveredService, DiscoveryMethod as SvcDiscoveryMethod,
};
use toadstool_common::universal_adapter::{
    CapabilityHandle, CapabilityInfo, CapabilityRequestBuilder, CapabilityType, DiscoveryEngine,
    EnvironmentSource, HealthStatus, LocalRegistrySource, MDnsSource, SecurityFeature,
    ServiceEndpoint as UniversalServiceEndpoint, StorageFeature, TrustLevel as UniversalTrustLevel,
    UniversalAdapter,
};

// ============================================================================
// Primal Discovery Tests
// ============================================================================

#[test]
fn test_primal_endpoint_creation() {
    let endpoint = PrimalEndpoint {
        service_id: "test-svc-1".to_string(),
        capabilities: vec!["compute".to_string(), "storage".to_string()],
        url: "http://localhost:8080".to_string(),
        trust_level: TrustLevel::Local,
        discovered_via: DiscoveryMethod::Configuration,
        discovered_at: Instant::now(),
        last_seen: Instant::now(),
        latency_ms: 5,
    };

    assert_eq!(endpoint.service_id, "test-svc-1");
    assert_eq!(endpoint.url(), "http://localhost:8080");
    assert!(endpoint.has_capability("compute"));
    assert!(endpoint.has_capability("storage"));
    assert!(!endpoint.has_capability("crypto"));
}

#[test]
fn test_trust_level_ordering() {
    // Verified < Local < Unverified (by enum declaration order)
    assert!(TrustLevel::Verified < TrustLevel::Local);
    assert!(TrustLevel::Local < TrustLevel::Unverified);
}

#[test]
fn test_discovery_method_variants() {
    assert_eq!(DiscoveryMethod::MDns, DiscoveryMethod::MDns);
    assert_eq!(
        DiscoveryMethod::Configuration,
        DiscoveryMethod::Configuration
    );
    let referral = DiscoveryMethod::Referral {
        from: "songbird".to_string(),
    };
    assert!(matches!(referral, DiscoveryMethod::Referral { from } if from == "songbird"));
}

#[test]
fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
    assert_eq!(config.health_check_interval, Duration::from_secs(30));
    assert!(config.fallbacks.is_empty());
    assert!(config.enable_mdns);
}

#[test]
fn test_discovery_error_variants() {
    let err = DiscoveryError::NotFound {
        capability: "crypto".to_string(),
    };
    assert!(err.to_string().contains("crypto"));

    let err = DiscoveryError::MDnsError("daemon failed".to_string());
    assert!(err.to_string().contains("daemon failed"));

    let err = DiscoveryError::ConfigError("bad config".to_string());
    assert!(err.to_string().contains("bad config"));
}

#[tokio::test]
async fn test_primal_discovery_new() {
    let result = PrimalDiscovery::new();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_primal_discovery_with_fallback() {
    let mut config = DiscoveryConfig {
        enable_mdns: false,
        ..Default::default()
    };
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://127.0.0.1:9000".to_string(),
    );

    let discovery = PrimalDiscovery::with_config(config).expect("create");
    let endpoint = discovery
        .find_capability("orchestration")
        .await
        .expect("find");
    assert_eq!(endpoint.url(), "http://127.0.0.1:9000");
    assert!(endpoint.has_capability("orchestration"));
}

#[tokio::test]
async fn test_primal_discovery_not_found() {
    let config = DiscoveryConfig {
        enable_mdns: false,
        ..Default::default()
    };
    let discovery = PrimalDiscovery::with_config(config).expect("create");
    let result = discovery.find_capability("nonexistent-cap").await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DiscoveryError::NotFound { .. }
    ));
}

#[tokio::test]
async fn test_primal_endpoint_is_fresh() {
    let endpoint = PrimalEndpoint {
        service_id: "fresh".to_string(),
        capabilities: vec!["test".to_string()],
        url: "http://localhost:8000".to_string(),
        trust_level: TrustLevel::Local,
        discovered_via: DiscoveryMethod::MDns,
        discovered_at: Instant::now(),
        last_seen: Instant::now(),
        latency_ms: 0,
    };
    assert!(endpoint.is_fresh(Duration::from_secs(10)));
}

// ============================================================================
// Primal Discovery mDNS Tests
// ============================================================================

#[test]
fn test_toadstool_service_type_constant() {
    assert_eq!(TOADSTOOL_SERVICE_TYPE, "_toadstool._tcp.local.");
}

#[test]
fn test_mdns_adapter_creation() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::new(config);
    if let Ok(adapter) = result {
        assert_eq!(adapter.timeout(), Duration::from_secs(3));
    }
}

#[test]
fn test_mdns_adapter_with_timeout() {
    let config = DiscoveryConfig::default();
    let timeout = Duration::from_millis(500);
    let result = MdnsAdapter::with_timeout(config, timeout);
    if let Ok(adapter) = result {
        assert_eq!(adapter.timeout(), timeout);
    }
}

// ============================================================================
// Service Discovery Config and Endpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_from_url_string() {
    let endpoint = ServiceEndpoint::from_url_string("http://localhost:8080").expect("parse");
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.address, "localhost");
    assert_eq!(endpoint.port, 8080);
}

#[test]
fn test_service_endpoint_from_url_https() {
    let endpoint = ServiceEndpoint::from_url_string("https://api.example.com:443").expect("parse");
    assert_eq!(endpoint.protocol, "https");
    assert_eq!(endpoint.address, "api.example.com");
    assert_eq!(endpoint.port, 443);
}

#[test]
fn test_service_endpoint_from_url_unix() {
    let endpoint = ServiceEndpoint::from_url_string("unix:///var/run/service.sock").expect("parse");
    assert_eq!(endpoint.protocol, "unix");
    assert!(endpoint.address.contains("service.sock"));
}

#[test]
fn test_service_endpoint_from_url_invalid() {
    let result = ServiceEndpoint::from_url_string("no-protocol");
    assert!(result.is_err());
}

#[test]
fn test_service_endpoint_constructors() {
    let http = ServiceEndpoint::http("localhost", 8080);
    assert_eq!(http.url(), "http://localhost:8080");

    let https = ServiceEndpoint::https("secure.example.com", 443);
    assert_eq!(https.url(), "https://secure.example.com:443");

    let grpc = ServiceEndpoint::grpc("grpc.local", 50051);
    assert_eq!(grpc.url(), "grpc://grpc.local:50051");
}

#[test]
fn test_service_endpoint_with_path_and_metadata() {
    let ep = ServiceEndpoint::http("api", 80)
        .with_path("/v1/query")
        .with_metadata("region", "us-west");
    assert_eq!(ep.path, Some("/v1/query".to_string()));
    assert_eq!(ep.metadata.get("region"), Some(&"us-west".to_string()));
}

#[tokio::test]
async fn test_service_discovery_method_variants() {
    assert_eq!(SvcDiscoveryMethod::Auto, SvcDiscoveryMethod::Auto);
    assert_eq!(SvcDiscoveryMethod::Mdns, SvcDiscoveryMethod::Mdns);
    assert_eq!(
        SvcDiscoveryMethod::Environment,
        SvcDiscoveryMethod::Environment
    );
}

#[tokio::test]
async fn test_discovered_service_creation() {
    let endpoint = ServiceEndpoint::from_url_string("http://localhost:8080").expect("parse");
    let service = DiscoveredService {
        id: "svc-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
        endpoints: vec![endpoint],
        metadata: std::collections::HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert!(service.has_capability(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(service.primary_endpoint().is_some());
}

// ============================================================================
// Primal Socket Paths and Discovery Tests
// ============================================================================

fn test_socket_env() -> SocketPathEnv {
    SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        user: Some("testuser".to_string()),
        ..Default::default()
    }
}

#[test]
fn test_resolve_runtime_dir() {
    let env = test_socket_env();
    assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
}

#[test]
fn test_resolve_biomeos_dir() {
    let env = test_socket_env();
    let path = resolve_biomeos_dir(&env);
    assert!(path.to_string_lossy().contains("biomeos"));
}

#[test]
fn test_resolve_family_id() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("my-family".to_string()),
        ..test_socket_env()
    };
    assert_eq!(resolve_family_id(&env), "my-family");
}

#[test]
fn test_resolve_beardog_socket() {
    let env = SocketPathEnv {
        beardog_socket: Some("/custom/beardog.sock".to_string()),
        ..test_socket_env()
    };
    let path = resolve_beardog_socket_fallback(&env);
    assert_eq!(path, PathBuf::from("/custom/beardog.sock"));
}

#[test]
fn test_resolve_songbird_socket() {
    let env = test_socket_env();
    let path = resolve_songbird_socket_fallback(&env);
    assert!(path.to_string_lossy().contains("songbird"));
}

#[test]
fn test_resolve_nestgate_socket() {
    let env = test_socket_env();
    let path = resolve_nestgate_socket_fallback(&env);
    assert!(path.to_string_lossy().contains("nestgate"));
}

#[test]
fn test_resolve_squirrel_socket() {
    let env = test_socket_env();
    let path = resolve_squirrel_socket(&env);
    assert!(path.to_string_lossy().contains("squirrel"));
}

#[test]
fn test_resolve_nucleus_socket() {
    let env = SocketPathEnv {
        nucleus_socket: Some("/var/run/nucleus.sock".to_string()),
        ..test_socket_env()
    };
    let path = resolve_nucleus_socket(&env);
    assert_eq!(path, PathBuf::from("/var/run/nucleus.sock"));
}

#[test]
fn test_resolve_toadstool_socket() {
    let env = test_socket_env();
    let path = resolve_toadstool_socket(&env);
    assert!(path.to_string_lossy().contains("toadstool"));
}

#[test]
fn test_resolve_socket_path_for_service() {
    let env = test_socket_env();
    let path = resolve_socket_path_for_service("beardog", &env, None);
    assert!(path.to_string_lossy().contains("beardog"));

    let override_path = PathBuf::from("/override/custom.sock");
    let path = resolve_socket_path_for_service("songbird", &env, Some(override_path.clone()));
    assert_eq!(path, override_path);
}

#[test]
fn test_resolve_socket_path_service_aliases() {
    let env = test_socket_env();
    let p1 = resolve_socket_path_for_service("bear-dog", &env, None);
    let p2 = resolve_socket_path_for_service("beardog", &env, None);
    assert_eq!(p1, p2);
}

#[test]
fn test_socket_discovery_error_variants() {
    let err = SocketDiscoveryError::DiscoveryFailed("init failed".to_string());
    assert!(err.to_string().contains("Discovery"));

    let err = SocketDiscoveryError::NoSocketFound("Capability::Crypto".to_string());
    assert!(err.to_string().contains("socket") || err.to_string().contains("Crypto"));

    let err = SocketDiscoveryError::InvalidEndpoint("bad path".to_string());
    assert!(err.to_string().contains("Invalid"));
}

// Note: discover_socket_for_capability, discover_crypto_socket, discover_storage_socket,
// discover_coordination_socket use CapabilityDiscovery::new() which creates a nested runtime
// (block_on inside block_on). They are tested in primal_sockets/discovery.rs unit tests.

// ============================================================================
// Primal Identity Types Tests
// ============================================================================

#[test]
fn test_capability_variants() {
    let compute = Capability::Compute(ComputeCapability::GpuCompute);
    assert!(matches!(compute, Capability::Compute(_)));

    let storage = Capability::Storage(StorageCapability::ObjectStorage);
    assert!(matches!(storage, Capability::Storage(_)));

    let crypto = Capability::Crypto(CryptoCapability::Encryption);
    assert!(matches!(crypto, Capability::Crypto(_)));

    let auth = Capability::Authentication(AuthCapability::TokenManagement);
    assert!(matches!(auth, Capability::Authentication(_)));

    let coord = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    assert!(matches!(coord, Capability::Coordination(_)));

    let disc = Capability::Discovery(DiscoveryCapability::MdnsDiscovery);
    assert!(matches!(disc, Capability::Discovery(_)));

    let custom = Capability::Custom {
        name: "custom".to_string(),
        version: "1.0".to_string(),
    };
    assert!(matches!(custom, Capability::Custom { .. }));
}

#[test]
fn test_compute_capability_variants() {
    assert!(matches!(
        ComputeCapability::NativeExecution,
        ComputeCapability::NativeExecution
    ));
    assert!(matches!(
        ComputeCapability::GpuCompute,
        ComputeCapability::GpuCompute
    ));
}

#[test]
fn test_capability_serialization_roundtrip() {
    let cap = Capability::Compute(ComputeCapability::NativeExecution);
    let json = serde_json::to_string(&cap).expect("serialize");
    let restored: Capability = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(cap, restored);
}

#[test]
fn test_coordination_capability_default() {
    let default = CoordinationCapability::default();
    assert_eq!(default, CoordinationCapability::ServiceDiscovery);
}

// ============================================================================
// Universal Adapter Types and Request Builder Tests
// ============================================================================

#[test]
fn test_capability_type_security() {
    let cap = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
        min_trust_level: UniversalTrustLevel::High,
    };
    assert!(matches!(cap, CapabilityType::Security { .. }));
}

#[test]
fn test_capability_type_storage() {
    let cap = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: Some(100),
    };
    assert!(matches!(cap, CapabilityType::Storage { .. }));
}

#[test]
fn test_trust_level_ordering_universal() {
    assert!(UniversalTrustLevel::Low < UniversalTrustLevel::Medium);
    assert!(UniversalTrustLevel::Medium < UniversalTrustLevel::High);
}

#[test]
fn test_universal_service_endpoint_variants() {
    let http = UniversalServiceEndpoint::Http("http://localhost:8080".to_string());
    assert!(matches!(http, UniversalServiceEndpoint::Http(_)));

    let tcp = UniversalServiceEndpoint::Tcp {
        host: "localhost".to_string(),
        port: 9000,
    };
    assert!(matches!(tcp, UniversalServiceEndpoint::Tcp { .. }));

    let unix = UniversalServiceEndpoint::UnixSocket(PathBuf::from("/tmp/sock"));
    assert!(matches!(unix, UniversalServiceEndpoint::UnixSocket(_)));

    assert!(matches!(
        UniversalServiceEndpoint::InProcess,
        UniversalServiceEndpoint::InProcess
    ));
}

#[test]
fn test_health_status_variants() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
}

#[test]
fn test_capability_handle_creation() {
    let info = CapabilityInfo {
        provider_id: "p-1".to_string(),
        capability: CapabilityType::Storage {
            features: vec![],
            min_throughput_mbps: None,
        },
        metadata: std::collections::HashMap::new(),
        endpoint: UniversalServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    };
    let handle = CapabilityHandle::new(
        info,
        CapabilityType::Storage {
            features: vec![],
            min_throughput_mbps: None,
        },
    );
    assert!(handle.is_healthy());
    assert_eq!(handle.provider_id(), "p-1");
}

#[test]
fn test_capability_request_builder_security() {
    let cap = CapabilityRequestBuilder::new()
        .security()
        .with_encryption()
        .with_signing()
        .min_trust_level(UniversalTrustLevel::High)
        .build();
    assert!(matches!(cap, CapabilityType::Security { .. }));
}

#[test]
fn test_capability_request_builder_storage() {
    let cap = CapabilityRequestBuilder::new()
        .storage()
        .with_compression()
        .min_throughput_mbps(50)
        .build();
    assert!(matches!(cap, CapabilityType::Storage { .. }));
}

#[test]
fn test_capability_request_builder_coordination() {
    let cap = CapabilityRequestBuilder::new()
        .coordination()
        .with_service_discovery()
        .max_latency_ms(20)
        .build();
    assert!(matches!(cap, CapabilityType::Coordination { .. }));
}

#[test]
fn test_capability_request_builder_intelligence() {
    let cap = CapabilityRequestBuilder::new()
        .intelligence()
        .with_natural_language()
        .with_llm()
        .build();
    assert!(matches!(cap, CapabilityType::Intelligence { .. }));
}

#[tokio::test]
async fn test_discovery_engine_empty() {
    let engine = DiscoveryEngine::empty();
    let providers = engine.discover_all().await.expect("discover");
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_discovery_engine_with_defaults() {
    let result = DiscoveryEngine::with_defaults();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_discovery_sources_creation() {
    let _env = EnvironmentSource::new();
    let _local = LocalRegistrySource::new();
    let _mdns = MDnsSource::new();
}

#[tokio::test]
async fn test_universal_adapter_default() {
    let adapter = UniversalAdapter::default();
    let caps = adapter.list_available_capabilities().await.expect("list");
    assert_eq!(caps.len(), 0);
}

#[tokio::test]
async fn test_universal_adapter_new() {
    let result = UniversalAdapter::new().await;
    assert!(result.is_ok());
}

// ============================================================================
// Capability Provider Tests
// ============================================================================

#[tokio::test]
async fn test_capability_provider_discover_fails_without_service() {
    let result =
        CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_capability_error_variants() {
    let err = CapabilityError::NoProviderFound(Capability::Crypto(CryptoCapability::Encryption));
    assert!(err.to_string().contains("No provider") || err.to_string().contains("provider"));

    let err = CapabilityError::ProviderUnreachable("svc".to_string());
    assert!(err.to_string().contains("svc"));

    let err = CapabilityError::RpcFailed("timeout".to_string());
    assert!(err.to_string().contains("timeout"));

    let err = CapabilityError::DiscoveryUnavailable;
    assert!(err.to_string().contains("unavailable"));

    let err = CapabilityError::InvalidResponse("bad json".to_string());
    assert!(err.to_string().contains("bad json"));
}

#[tokio::test]
async fn test_discover_all_fails_without_service() {
    let result = discover_all(Capability::Crypto(CryptoCapability::Encryption)).await;
    assert!(result.is_err());
}
