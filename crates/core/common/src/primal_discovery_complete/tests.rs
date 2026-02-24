//! Tests for primal discovery complete module

use super::*;
use crate::primal_identity::{ComputeCapability, CoordinationCapability, StorageCapability};
use async_trait::async_trait;
use std::time::Duration;

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Mock DiscoveryClient for testing mDNS code paths without network
struct MockDiscoveryClient {
    services: std::sync::RwLock<Option<Vec<DiscoveredService>>>,
    error: std::sync::RwLock<Option<String>>,
}

#[async_trait]
impl crate::runtime_discovery::DiscoveryClient for MockDiscoveryClient {
    async fn discover_by_capability(
        &self,
        _capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        if let Ok(guard) = self.error.read() {
            if let Some(ref msg) = *guard {
                return Err(ToadStoolError::runtime(msg.clone()));
            }
        }
        if let Ok(guard) = self.services.read() {
            if let Some(ref svcs) = *guard {
                return Ok(svcs.clone());
            }
        }
        Ok(vec![])
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        Ok(vec![])
    }

    async fn register_service(&self, _service: &DiscoveredService) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn deregister_service(&self, _service_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn health_check(&self, _service_id: &str) -> ToadStoolResult<bool> {
        Ok(true)
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_discovery_with_fallback() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://localhost:9999".to_string(),
    );
    config.enable_mdns = false; // Disable mDNS for this test

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    let result = engine.discover_by_capability(&capability).await;

    assert!(result.is_ok(), "Should find orchestration via fallback");
    let services = result.expect("Services should be present");
    assert!(!services.is_empty(), "Should have at least one service");
    assert_eq!(services[0].endpoints[0].port, 9999);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cache_freshness() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let engine = PrimalDiscoveryEngine::new(None)
        .await
        .expect("Failed to create engine");

    let service = DiscoveredService {
        id: Some("test-service".to_string()),
        capabilities: vec![Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        )],
        endpoints: vec![crate::primal_identity::ServiceEndpoint {
            protocol: "http".to_string(),
            address: "localhost".to_string(),
            port: 8080,
            path: Some("/".to_string()),
            metadata: HashMap::new(),
        }],
        healthy: true,
        metadata: HashMap::new(),
    };

    engine.cache_service("orchestration", service.clone()).await;

    // Should be in cache
    let cached = engine.get_from_cache("orchestration").await;
    assert!(cached.is_some(), "Service should be cached");
    assert!(
        cached.unwrap().is_fresh(Duration::from_secs(300)),
        "Service should be fresh"
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cache_stats() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let engine = PrimalDiscoveryEngine::new(None)
        .await
        .expect("Failed to create engine");

    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 0, "Cache should be empty initially");

    let service = DiscoveredService {
        id: Some("test-service".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    engine.cache_service("test", service).await;

    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 1, "Cache should have one entry");
    assert_eq!(stats.fresh_entries, 1, "Entry should be fresh");
}

#[tokio::test]
async fn test_with_config_require_mdns_no_client() {
    let config = DiscoveryConfig {
        require_mdns: true,
        fallbacks: HashMap::new(),
        ..Default::default()
    };

    let result = PrimalDiscoveryEngine::with_config(None, config).await;

    assert!(result.is_err());
    let err_msg = result.err().unwrap().to_string();
    assert!(err_msg.contains("mDNS client required"));
}

#[tokio::test]
async fn test_discover_by_capability_not_found() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false; // Prevent flaky failure from parallel env-var test

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Compute(ComputeCapability::NativeExecution);
    let result = engine.discover_by_capability(&capability).await;

    assert!(result.is_err());
    let err_msg = result.err().unwrap().to_string();
    assert!(err_msg.contains("No service found"));
}

#[tokio::test]
async fn test_clear_cache() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://localhost:9998".to_string(),
    );
    config.enable_mdns = false;
    config.require_mdns = false; // Prevent flaky failure from parallel env-var test

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    let _ = engine.discover_by_capability(&capability).await.unwrap();

    let stats_before = engine.cache_stats().await;
    assert_eq!(stats_before.total_entries, 1, "Should have cached entry");

    engine.clear_cache().await;

    let stats_after = engine.cache_stats().await;
    assert_eq!(stats_after.total_entries, 0, "Cache should be empty");
}

#[tokio::test]
async fn test_create_fallback_service_https() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "storage".to_string(),
        "https://example.com:443/api".to_string(),
    );
    config.enable_mdns = false;
    config.require_mdns = false; // Prevent flaky failure from parallel env-var test

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Storage(StorageCapability::ObjectStorage);
    let services = engine
        .discover_by_capability(&capability)
        .await
        .expect("Should find fallback");

    assert_eq!(services[0].endpoints[0].protocol, "https");
    assert_eq!(services[0].endpoints[0].address, "example.com");
    assert_eq!(services[0].endpoints[0].port, 443);
}

#[tokio::test]
async fn test_create_fallback_service_socket_addr() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("compute".to_string(), "127.0.0.1:9090".to_string());
    config.enable_mdns = false;
    config.require_mdns = false; // Prevent flaky failure from parallel env-var test

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Compute(ComputeCapability::NativeExecution);
    let services = engine
        .discover_by_capability(&capability)
        .await
        .expect("Should find fallback");

    assert_eq!(services[0].endpoints[0].address, "127.0.0.1");
    assert_eq!(services[0].endpoints[0].port, 9090);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_discovery_config_default_with_env() {
    let _guard = ENV_MUTEX.lock().unwrap();
    std::env::set_var("TOADSTOOL_MDNS_ENABLE", "false");
    std::env::set_var("TOADSTOOL_MDNS_REQUIRE", "true");

    let config = DiscoveryConfig::default();

    assert!(!config.enable_mdns);
    assert!(config.require_mdns);

    std::env::remove_var("TOADSTOOL_MDNS_ENABLE");
    std::env::remove_var("TOADSTOOL_MDNS_REQUIRE");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cache_stats_stale_entries() {
    let service = DiscoveredService {
        id: Some("stale-service".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    // Duration::ZERO TTL means every entry is immediately stale —
    // is_fresh() checks `elapsed < ttl`, and elapsed >= ZERO always, so
    // with ttl=ZERO `elapsed < ZERO` is always false. No sleep required.
    let config = DiscoveryConfig {
        cache_ttl: Duration::ZERO,
        ..Default::default()
    };

    let engine_with_short_ttl = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");
    engine_with_short_ttl
        .cache_service("stale_key", service.clone())
        .await;

    let stats = engine_with_short_ttl.cache_stats().await;
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.fresh_entries, 0, "Entry should be stale");
    assert_eq!(stats.stale_entries, 1);
}

#[tokio::test]
async fn test_capability_to_string_variants() {
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery
        )),
        "orchestration"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Compute(
            ComputeCapability::NativeExecution
        )),
        "compute"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Storage(
            StorageCapability::ObjectStorage
        )),
        "storage"
    );
}

#[tokio::test]
async fn test_capability_to_string_crypto_auth_discovery_custom() {
    use crate::primal_identity::{AuthCapability, CryptoCapability, DiscoveryCapability};

    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Crypto(
            CryptoCapability::Encryption
        )),
        "crypto"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Authentication(
            AuthCapability::UserAuth
        )),
        "authentication"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Discovery(
            DiscoveryCapability::MdnsDiscovery
        )),
        "discovery"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Custom {
            name: "custom-cap".to_string(),
            version: "1.0".to_string(),
        }),
        "custom-cap"
    );
}

#[tokio::test]
async fn test_create_fallback_service_http() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("test".to_string(), "http://127.0.0.1:8080".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let capability = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine
        .discover_by_capability(&capability)
        .await
        .expect("Should find fallback");

    assert_eq!(services[0].endpoints[0].protocol, "http");
    assert_eq!(services[0].endpoints[0].address, "127.0.0.1");
    assert_eq!(services[0].endpoints[0].port, 8080);
    assert!(services[0]
        .metadata
        .get("source")
        .map(|s| s == "configuration")
        .unwrap_or(false));
}

#[tokio::test]
async fn test_cache_stats_empty() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.fresh_entries, 0);
    assert_eq!(stats.stale_entries, 0);
}

#[tokio::test]
async fn test_discovery_config_default_fallbacks() {
    let config = DiscoveryConfig::default();
    assert!(config.cache_ttl.as_secs() > 0);
    assert!(config.health_check_interval.as_secs() > 0);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cached_endpoint_is_fresh() {
    let service = DiscoveredService {
        id: Some("fresh".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    engine.cache_service("fresh_key", service).await;
    let cached = engine.get_from_cache("fresh_key").await;
    assert!(cached.is_some());
    assert!(cached.unwrap().is_fresh(Duration::from_secs(300)));
}

#[tokio::test]
async fn test_mdns_discovery_success_path() {
    let mdns_service = DiscoveredService {
        id: Some("mdns-svc".to_string()),
        capabilities: vec![Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        )],
        endpoints: vec![crate::primal_identity::ServiceEndpoint {
            protocol: "http".to_string(),
            address: "mdns-host".to_string(),
            port: 9000,
            path: Some("/".to_string()),
            metadata: HashMap::new(),
        }],
        healthy: true,
        metadata: HashMap::new(),
    };

    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(Some(vec![mdns_service])),
        error: std::sync::RwLock::new(None),
    };

    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("Failed to create engine");

    let result = engine
        .discover_by_capability(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        ))
        .await;

    assert!(result.is_ok());
    let services = result.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].endpoints[0].address, "mdns-host");
    assert_eq!(services[0].endpoints[0].port, 9000);
}

#[tokio::test]
async fn test_mdns_discovery_empty_falls_to_fallback() {
    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(Some(vec![])),
        error: std::sync::RwLock::new(None),
    };

    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://fallback:8888".to_string(),
    );
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("Failed to create engine");

    let result = engine
        .discover_by_capability(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        ))
        .await;

    assert!(result.is_ok());
    let services = result.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].endpoints[0].address, "fallback");
    assert_eq!(services[0].endpoints[0].port, 8888);
}

#[tokio::test]
async fn test_mdns_discovery_error_falls_to_fallback() {
    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(None),
        error: std::sync::RwLock::new(Some("mdns network error".to_string())),
    };

    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("compute".to_string(), "http://127.0.0.1:7777".to_string());
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("Failed to create engine");

    let result = engine
        .discover_by_capability(&Capability::Compute(ComputeCapability::NativeExecution))
        .await;

    assert!(result.is_ok());
    let services = result.unwrap();
    assert_eq!(services[0].endpoints[0].address, "127.0.0.1");
    assert_eq!(services[0].endpoints[0].port, 7777);
}

#[tokio::test]
async fn test_cache_hit_returns_cached_service() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://localhost:9997".to_string(),
    );
    config.enable_mdns = false;
    config.require_mdns = false;
    config.cache_ttl = Duration::from_secs(300);

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let cap = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    let first = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(first[0].endpoints[0].port, 9997);

    let second = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].endpoints[0].port, 9997);
}

#[tokio::test]
async fn test_create_fallback_service_malformed_url_uses_defaults() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("test".to_string(), "not-a-valid-url".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let cap = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].endpoints[0].address, "localhost");
    assert_eq!(
        services[0].endpoints[0].port,
        crate::constants::network::DEFAULT_HTTP_PORT
    );
}

#[tokio::test]
async fn test_create_fallback_service_http_no_port_uses_80() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("test".to_string(), "http://example.com/path".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Failed to create engine");

    let cap = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(services[0].endpoints[0].port, 80);
    assert_eq!(services[0].endpoints[0].address, "example.com");
}

#[tokio::test]
async fn test_with_config_enable_mdns_no_fallbacks_warn_path() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;
    config.fallbacks.clear();

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("Should create even with no mDNS and no fallbacks");

    let cap = Capability::Compute(ComputeCapability::NativeExecution);
    let result = engine.discover_by_capability(&cap).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_with_config_mdns_enabled_with_client() {
    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(None),
        error: std::sync::RwLock::new(None),
    };
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("Should create when mDNS client provided");
    let _ = engine;
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cache_stats_multiple_entries() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create engine");

    for i in 0..3 {
        let service = DiscoveredService {
            id: Some(format!("svc-{i}")),
            capabilities: vec![],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };
        engine.cache_service(&format!("cap-{i}"), service).await;
    }

    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 3);
}

#[tokio::test]
async fn test_discover_by_capability_not_found_no_fallback() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "nonexistent-cap".to_string(),
        version: "1.0".to_string(),
    };
    let result = engine.discover_by_capability(&cap).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("No service found"));
    assert!(err_msg.contains("nonexistent-cap"));
}

#[tokio::test]
async fn test_create_fallback_service_url_with_path() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "api".to_string(),
        "https://api.example.com:443/v1/endpoint".to_string(),
    );
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "api".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(
        services[0].endpoints[0].path,
        Some("/v1/endpoint".to_string())
    );
    assert_eq!(services[0].endpoints[0].port, 443);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cached_endpoint_stale_after_ttl() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;
    config.cache_ttl = Duration::from_nanos(1);

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let service = DiscoveredService {
        id: Some("stale".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    engine.cache_service("stale_key", service).await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.fresh_entries, 0);
    assert_eq!(stats.stale_entries, 1);
}

#[tokio::test]
async fn test_discovery_config_cache_and_health_intervals() {
    let config = DiscoveryConfig::default();
    assert!(config.cache_ttl.as_secs() >= 1);
    assert!(config.health_check_interval.as_secs() >= 1);
}

#[tokio::test]
async fn test_discovered_service_metadata_source() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("meta".to_string(), "http://127.0.0.1:9000".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "meta".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(services[0].metadata.get("source").unwrap(), "configuration");
    assert!(services[0].healthy);
}

#[tokio::test]
async fn test_mdns_discovery_empty_no_fallback_fails() {
    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(Some(vec![])),
        error: std::sync::RwLock::new(None),
    };
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("create");

    let cap = Capability::Compute(ComputeCapability::NativeExecution);
    let result = engine.discover_by_capability(&cap).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mdns_discovery_error_no_fallback_fails() {
    let mock = MockDiscoveryClient {
        services: std::sync::RwLock::new(None),
        error: std::sync::RwLock::new(Some("network failure".to_string())),
    };
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = true;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(Some(std::sync::Arc::new(mock)), config)
        .await
        .expect("create");

    let cap = Capability::Storage(StorageCapability::ObjectStorage);
    let result = engine.discover_by_capability(&cap).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_fallback_service_unknown_protocol_uses_http() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("test".to_string(), "ftp://example.com:21/path".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(services.len(), 1);
    assert!(!services[0].endpoints[0].protocol.is_empty());
}

#[tokio::test]
async fn test_capability_to_string_all_variants() {
    use crate::primal_identity::{AuthCapability, CryptoCapability, DiscoveryCapability};

    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery
        )),
        "orchestration"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Compute(
            ComputeCapability::NativeExecution
        )),
        "compute"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Storage(
            StorageCapability::ObjectStorage
        )),
        "storage"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Crypto(
            CryptoCapability::Encryption
        )),
        "crypto"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Authentication(
            AuthCapability::UserAuth
        )),
        "authentication"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Discovery(
            DiscoveryCapability::MdnsDiscovery
        )),
        "discovery"
    );
    assert_eq!(
        PrimalDiscoveryEngine::capability_to_string(&Capability::Custom {
            name: "my-cap".to_string(),
            version: "2.0".to_string(),
        }),
        "my-cap"
    );
}

#[tokio::test]
async fn test_cache_stats_after_clear() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("cap".to_string(), "http://127.0.0.1:9000".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let _ = engine
        .discover_by_capability(&Capability::Custom {
            name: "cap".to_string(),
            version: "1.0".to_string(),
        })
        .await
        .unwrap();
    engine.clear_cache().await;
    let stats = engine.cache_stats().await;
    assert_eq!(stats.total_entries, 0);
}

#[tokio::test]
async fn test_not_found_error_includes_capability_name() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "unique-cap-x".to_string(),
        version: "1.0".to_string(),
    };
    let result = engine.discover_by_capability(&cap).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unique-cap-x"));
    assert!(err_msg.to_lowercase().contains("no service"));
}

#[tokio::test]
async fn test_fallback_service_metadata_source() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("meta-cap".to_string(), "http://127.0.0.1:8888".to_string());
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    let cap = Capability::Custom {
        name: "meta-cap".to_string(),
        version: "1.0".to_string(),
    };
    let services = engine.discover_by_capability(&cap).await.unwrap();
    assert_eq!(services[0].metadata.get("source").unwrap(), "configuration");
    assert!(services[0].healthy);
}

#[test]
fn test_cache_stats_debug_clone() {
    let stats = super::CacheStats {
        total_entries: 5,
        fresh_entries: 3,
        stale_entries: 2,
    };
    let debug_str = format!("{:?}", stats);
    assert!(debug_str.contains("total_entries"));
    assert!(debug_str.contains("5"));
    let cloned = stats.clone();
    assert_eq!(cloned.total_entries, stats.total_entries);
    assert_eq!(cloned.fresh_entries, stats.fresh_entries);
    assert_eq!(cloned.stale_entries, stats.stale_entries);
}

#[test]
fn test_discovery_config_default() {
    let config = super::DiscoveryConfig::default();
    assert!(config.cache_ttl.as_secs() >= 1);
    assert!(config.health_check_interval.as_secs() >= 1);
}

#[test]
fn test_discovery_config_custom() {
    use std::time::Duration;
    let config = super::DiscoveryConfig {
        cache_ttl: Duration::from_secs(600),
        health_check_interval: Duration::from_secs(60),
        fallbacks: std::collections::HashMap::new(),
        enable_mdns: false,
        require_mdns: false,
    };
    assert_eq!(config.cache_ttl.as_secs(), 600);
    assert_eq!(config.health_check_interval.as_secs(), 60);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cached_endpoint_is_fresh_with_nonzero_ttl() {
    let service = DiscoveredService {
        id: Some("fresh".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;
    config.cache_ttl = std::time::Duration::from_secs(600);

    let engine = PrimalDiscoveryEngine::with_config(None, config)
        .await
        .expect("create");

    engine.cache_service("fresh_key", service).await;
    let cached = engine.get_from_cache("fresh_key").await;
    assert!(cached.is_some());
    assert!(cached
        .unwrap()
        .is_fresh(std::time::Duration::from_secs(600)));
}
