// SPDX-License-Identifier: AGPL-3.0-only
//! Cache, config, and capability tests

use super::super::*;
use crate::primal_identity::{ComputeCapability, CoordinationCapability};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_cache_freshness() {
    let engine = PrimalDiscoveryEngine::new(None).expect("Failed to create engine");

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

    let cached = engine.get_from_cache("orchestration").await;
    assert!(cached.is_some(), "Service should be cached");
    assert!(
        cached.unwrap().is_fresh(Duration::from_secs(300)),
        "Service should be fresh"
    );
}

#[tokio::test]
async fn test_cache_stats() {
    let engine = PrimalDiscoveryEngine::new(None).expect("Failed to create engine");

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
async fn test_clear_cache() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://localhost:9998".to_string(),
    );
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("Failed to create engine");

    let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    let _ = engine.discover_by_capability(&capability).await.unwrap();

    let stats_before = engine.cache_stats().await;
    assert_eq!(stats_before.total_entries, 1, "Should have cached entry");

    engine.clear_cache().await;

    let stats_after = engine.cache_stats().await;
    assert_eq!(stats_after.total_entries, 0, "Cache should be empty");
}

#[tokio::test]
async fn test_discovery_config_default_with_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_MDNS_ENABLE", Some("false")),
            ("TOADSTOOL_MDNS_REQUIRE", Some("true")),
        ],
        || {
            let config = DiscoveryConfig::default();
            assert!(!config.enable_mdns);
            assert!(config.require_mdns);
        },
    );
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

    let config = DiscoveryConfig {
        cache_ttl: Duration::ZERO,
        ..Default::default()
    };

    let engine_with_short_ttl =
        PrimalDiscoveryEngine::with_config(None, config).expect("Failed to create engine");
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
            crate::primal_identity::StorageCapability::ObjectStorage
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
async fn test_cache_stats_empty() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("Failed to create engine");

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

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("Failed to create engine");

    engine.cache_service("fresh_key", service).await;
    let cached = engine.get_from_cache("fresh_key").await;
    assert!(cached.is_some());
    assert!(cached.unwrap().is_fresh(Duration::from_secs(300)));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_cache_stats_multiple_entries() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("create engine");

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
#[allow(clippy::await_holding_lock)]
async fn test_cached_endpoint_stale_after_ttl() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.clear();
    config.enable_mdns = false;
    config.require_mdns = false;
    // Duration::ZERO makes cache immediately stale; no sleep needed (cache uses std::time::Instant,
    // which tokio virtual time cannot advance)
    config.cache_ttl = Duration::ZERO;

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("create");

    let service = DiscoveredService {
        id: Some("stale".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    engine.cache_service("stale_key", service).await;

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
            crate::primal_identity::StorageCapability::ObjectStorage
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

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("create");

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

#[test]
fn test_cache_stats_debug_clone() {
    let stats = super::super::CacheStats {
        total_entries: 5,
        fresh_entries: 3,
        stale_entries: 2,
    };
    let debug_str = format!("{stats:?}");
    assert!(debug_str.contains("total_entries"));
    assert!(debug_str.contains('5'));
    let cloned = stats.clone();
    assert_eq!(cloned.total_entries, stats.total_entries);
    assert_eq!(cloned.fresh_entries, stats.fresh_entries);
    assert_eq!(cloned.stale_entries, stats.stale_entries);
}

#[test]
fn test_discovery_config_default() {
    let config = super::super::DiscoveryConfig::default();
    assert!(config.cache_ttl.as_secs() >= 1);
    assert!(config.health_check_interval.as_secs() >= 1);
}

#[test]
fn test_discovery_config_custom() {
    use std::time::Duration;
    let config = super::super::DiscoveryConfig {
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

    let engine = PrimalDiscoveryEngine::with_config(None, config).expect("create");

    engine.cache_service("fresh_key", service).await;
    let cached = engine.get_from_cache("fresh_key").await;
    assert!(cached.is_some());
    assert!(
        cached
            .unwrap()
            .is_fresh(std::time::Duration::from_secs(600))
    );
}
