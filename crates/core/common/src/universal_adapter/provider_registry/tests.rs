// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use super::super::capability_types::{CapabilityInfo, CapabilityType, HealthStatus};
use super::ProviderRegistry;
use crate::universal_adapter::{
    ComputeFeature, CoordinationFeature, IntelligenceFeature, ModelType, SecurityFeature,
    ServiceEndpoint, StorageFeature, TrustLevel,
};

fn create_test_security_provider() -> CapabilityInfo {
    CapabilityInfo {
        provider_id: "test-security-1".to_string(),
        capability: CapabilityType::Security {
            features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
            min_trust_level: TrustLevel::High,
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

fn create_storage_provider(throughput: Option<u64>) -> CapabilityInfo {
    CapabilityInfo {
        provider_id: "test-storage-1".to_string(),
        capability: CapabilityType::Storage {
            features: vec![StorageFeature::Compression, StorageFeature::Encryption],
            min_throughput_mbps: throughput,
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

fn create_coordination_provider(latency: Option<u64>) -> CapabilityInfo {
    CapabilityInfo {
        provider_id: "test-coord-1".to_string(),
        capability: CapabilityType::Coordination {
            features: vec![CoordinationFeature::ServiceDiscovery],
            max_latency_ms: latency,
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

fn create_compute_provider(memory_gb: Option<f64>) -> CapabilityInfo {
    CapabilityInfo {
        provider_id: "test-compute-1".to_string(),
        capability: CapabilityType::Compute {
            features: vec![ComputeFeature::GPU],
            min_memory_gb: memory_gb,
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

fn create_intelligence_provider() -> CapabilityInfo {
    CapabilityInfo {
        provider_id: "test-intel-1".to_string(),
        capability: CapabilityType::Intelligence {
            features: vec![IntelligenceFeature::NaturalLanguage],
            model_types: vec![ModelType::LLM],
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

fn security_provider_with_id(
    id: &str,
    features: Vec<SecurityFeature>,
    trust: TrustLevel,
) -> CapabilityInfo {
    CapabilityInfo {
        provider_id: id.to_string(),
        capability: CapabilityType::Security {
            features,
            min_trust_level: trust,
        },
        metadata: HashMap::new(),
        endpoint: ServiceEndpoint::InProcess,
        health: HealthStatus::Healthy,
    }
}

#[test]
fn test_registry_creation() {
    let registry = ProviderRegistry::new();
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_provider_registration() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();

    registry.register(provider).unwrap();
    assert_eq!(registry.provider_count(), 1);
}

#[test]
fn test_provider_unregistration() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();
    let provider_id = provider.provider_id.clone();

    registry.register(provider).unwrap();
    assert_eq!(registry.provider_count(), 1);

    registry.unregister(&provider_id).unwrap();
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_capability_matching() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();
    registry.register(provider).unwrap();

    // Request with matching features
    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };

    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());
}

#[test]
fn test_capability_no_match() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();
    registry.register(provider).unwrap();

    // Request with incompatible trust level
    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Maximum, // Provider only has High
    };

    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_none(), "Should not match - trust level too high");
}

#[test]
fn test_health_update() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();
    let provider_id = provider.provider_id.clone();

    registry.register(provider).unwrap();
    registry.update_health(&provider_id, HealthStatus::Degraded);

    let info = registry.get_provider(&provider_id).unwrap();
    assert_eq!(info.health, HealthStatus::Degraded);
}

#[test]
fn test_success_failure_tracking() {
    let mut registry = ProviderRegistry::new();
    let provider = create_test_security_provider();
    let provider_id = provider.provider_id.clone();

    registry.register(provider).unwrap();

    registry.record_success(&provider_id);
    registry.record_success(&provider_id);
    registry.record_failure(&provider_id);

    // Verify counts are tracked (internal state)
    assert!(registry.get_provider(&provider_id).is_some());
}

#[test]
fn test_list_capabilities() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_test_security_provider()).unwrap();

    let caps = registry.list_capabilities();
    assert_eq!(caps.len(), 1);
}

#[test]
fn test_clear_registry() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_test_security_provider()).unwrap();
    assert_eq!(registry.provider_count(), 1);

    registry.clear();
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_registry_default() {
    let registry = ProviderRegistry::default();
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_storage_capability_matching_throughput() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_storage_provider(Some(200)))
        .unwrap();

    let request = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: Some(100),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());

    let request_too_high = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: Some(500),
    };
    let no_match = registry.find_best_match(&request_too_high).unwrap();
    assert!(no_match.is_none());
}

#[test]
fn test_storage_capability_request_no_throughput() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_storage_provider(Some(100)))
        .unwrap();

    let request = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: None,
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());
}

#[test]
fn test_storage_provider_no_throughput_fails_when_required() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_storage_provider(None)).unwrap();

    let request = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: Some(100),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_none());
}

#[test]
fn test_coordination_capability_matching_latency() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_coordination_provider(Some(50)))
        .unwrap();

    let request = CapabilityType::Coordination {
        features: vec![CoordinationFeature::ServiceDiscovery],
        max_latency_ms: Some(100),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());

    let request_too_strict = CapabilityType::Coordination {
        features: vec![CoordinationFeature::ServiceDiscovery],
        max_latency_ms: Some(10),
    };
    let no_match = registry.find_best_match(&request_too_strict).unwrap();
    assert!(no_match.is_none());
}

#[test]
fn test_compute_capability_matching_memory() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_compute_provider(Some(16.0)))
        .unwrap();

    let request = CapabilityType::Compute {
        features: vec![ComputeFeature::GPU],
        min_memory_gb: Some(8.0),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());

    let request_too_high = CapabilityType::Compute {
        features: vec![ComputeFeature::GPU],
        min_memory_gb: Some(32.0),
    };
    let no_match = registry.find_best_match(&request_too_high).unwrap();
    assert!(no_match.is_none());
}

#[test]
fn test_intelligence_capability_matching() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_intelligence_provider()).unwrap();

    let request = CapabilityType::Intelligence {
        features: vec![IntelligenceFeature::NaturalLanguage],
        model_types: vec![ModelType::LLM],
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());

    let request_missing_model = CapabilityType::Intelligence {
        features: vec![IntelligenceFeature::NaturalLanguage],
        model_types: vec![ModelType::Vision],
    };
    let no_match = registry.find_best_match(&request_missing_model).unwrap();
    assert!(no_match.is_none());
}

#[test]
fn test_network_capability_matching() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(CapabilityInfo {
            provider_id: "net-1".to_string(),
            capability: CapabilityType::Network {
                features: vec![],
                min_bandwidth_mbps: None,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        })
        .unwrap();

    let request = CapabilityType::Network {
        features: vec![],
        min_bandwidth_mbps: Some(100),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());
}

#[test]
fn test_monitoring_capability_matching() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(CapabilityInfo {
            provider_id: "mon-1".to_string(),
            capability: CapabilityType::Monitoring {
                features: vec![],
                retention_days: None,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        })
        .unwrap();

    let request = CapabilityType::Monitoring {
        features: vec![],
        retention_days: Some(30),
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());
}

#[test]
fn test_capability_cross_type_no_match() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_test_security_provider()).unwrap();

    let request = CapabilityType::Storage {
        features: vec![StorageFeature::Compression],
        min_throughput_mbps: None,
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_none());
}

#[test]
fn test_unhealthy_provider_filtered_out() {
    let mut registry = ProviderRegistry::new();
    let mut provider = create_test_security_provider();
    provider.health = HealthStatus::Unhealthy;
    registry.register(provider).unwrap();

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_none());
}

#[test]
fn test_unknown_health_provider_matches() {
    let mut registry = ProviderRegistry::new();
    let mut provider = create_test_security_provider();
    provider.health = HealthStatus::Unknown;
    registry.register(provider).unwrap();

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    let matched = registry.find_best_match(&request).unwrap();
    assert!(matched.is_some());
}

#[test]
fn test_update_health_nonexistent_no_panic() {
    let mut registry = ProviderRegistry::new();
    registry.update_health("nonexistent", HealthStatus::Degraded);
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_record_success_nonexistent_no_panic() {
    let mut registry = ProviderRegistry::new();
    registry.record_success("nonexistent");
}

#[test]
fn test_record_failure_nonexistent_no_panic() {
    let mut registry = ProviderRegistry::new();
    registry.record_failure("nonexistent");
}

#[test]
fn test_get_provider_nonexistent() {
    let registry = ProviderRegistry::new();
    assert!(registry.get_provider("missing").is_none());
}

#[test]
fn test_register_duplicate_id_overwrites() {
    let mut registry = ProviderRegistry::new();
    let mut first = security_provider_with_id(
        "dup-id",
        vec![SecurityFeature::Encryption],
        TrustLevel::Medium,
    );
    first.metadata.insert("v".to_string(), "1".to_string());
    registry.register(first).unwrap();

    let mut second = security_provider_with_id(
        "dup-id",
        vec![SecurityFeature::Encryption, SecurityFeature::Signing],
        TrustLevel::High,
    );
    second.metadata.insert("v".to_string(), "2".to_string());
    registry.register(second).unwrap();

    assert_eq!(registry.provider_count(), 1);
    let info = registry.get_provider("dup-id").unwrap();
    assert_eq!(info.metadata.get("v"), Some(&"2".to_string()));
    match &info.capability {
        CapabilityType::Security { features, .. } => {
            assert!(features.contains(&SecurityFeature::Signing));
        }
        _ => panic!("expected Security capability"),
    }
}

#[test]
fn test_unregister_nonexistent_is_ok() {
    let mut registry = ProviderRegistry::new();
    registry.unregister("never-registered").unwrap();
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_find_best_match_prefers_healthy_over_unknown() {
    let mut registry = ProviderRegistry::new();
    let mut unknown = security_provider_with_id(
        "sec-unknown",
        vec![SecurityFeature::Encryption],
        TrustLevel::High,
    );
    unknown.health = HealthStatus::Unknown;
    registry.register(unknown).unwrap();

    let healthy = security_provider_with_id(
        "sec-healthy",
        vec![SecurityFeature::Encryption],
        TrustLevel::High,
    );
    registry.register(healthy).unwrap();

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    let best = registry.find_best_match(&request).unwrap().unwrap();
    assert_eq!(best.provider_id, "sec-healthy");
}

#[test]
fn test_find_best_match_prefers_lower_failure_rate() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(security_provider_with_id(
            "reliable",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();
    registry
        .register(security_provider_with_id(
            "flaky",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();

    for _ in 0..10 {
        registry.record_success("reliable");
    }
    for _ in 0..10 {
        registry.record_failure("flaky");
    }

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    let best = registry.find_best_match(&request).unwrap().unwrap();
    assert_eq!(best.provider_id, "reliable");
}

#[test]
fn test_matches_security_requested_feature_not_available() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(security_provider_with_id(
            "enc-only",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
        min_trust_level: TrustLevel::Medium,
    };
    assert!(registry.find_best_match(&request).unwrap().is_none());
}

#[test]
fn test_matches_storage_requested_feature_not_available() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(CapabilityInfo {
            provider_id: "st-1".to_string(),
            capability: CapabilityType::Storage {
                features: vec![StorageFeature::Compression],
                min_throughput_mbps: Some(100),
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        })
        .unwrap();

    let request = CapabilityType::Storage {
        features: vec![StorageFeature::Compression, StorageFeature::Encryption],
        min_throughput_mbps: None,
    };
    assert!(registry.find_best_match(&request).unwrap().is_none());
}

#[test]
fn test_matches_coordination_requested_feature_not_available() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_coordination_provider(Some(50)))
        .unwrap();

    let request = CapabilityType::Coordination {
        features: vec![
            CoordinationFeature::ServiceDiscovery,
            CoordinationFeature::LoadBalancing,
        ],
        max_latency_ms: Some(100),
    };
    assert!(registry.find_best_match(&request).unwrap().is_none());
}

#[test]
fn test_matches_compute_requested_feature_not_available() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(create_compute_provider(Some(32.0)))
        .unwrap();

    let request = CapabilityType::Compute {
        features: vec![ComputeFeature::GPU, ComputeFeature::MultiCore],
        min_memory_gb: Some(8.0),
    };
    assert!(registry.find_best_match(&request).unwrap().is_none());
}

#[test]
fn test_update_health_transitions_round_trip() {
    let mut registry = ProviderRegistry::new();
    let id = "health-id";
    registry
        .register(security_provider_with_id(
            id,
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();
    assert_eq!(
        registry.get_provider(id).unwrap().health,
        HealthStatus::Healthy
    );

    registry.update_health(id, HealthStatus::Degraded);
    assert_eq!(
        registry.get_provider(id).unwrap().health,
        HealthStatus::Degraded
    );

    registry.update_health(id, HealthStatus::Healthy);
    assert_eq!(
        registry.get_provider(id).unwrap().health,
        HealthStatus::Healthy
    );
}

#[test]
fn test_record_success_and_failure_counts_affect_ranking() {
    let mut registry = ProviderRegistry::new();
    registry
        .register(security_provider_with_id(
            "a",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();
    registry
        .register(security_provider_with_id(
            "b",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();

    registry.record_success("a");
    registry.record_success("a");
    registry.record_failure("b");
    registry.record_failure("b");

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    let best = registry.find_best_match(&request).unwrap().unwrap();
    assert_eq!(best.provider_id, "a");
}

#[test]
fn test_list_capabilities_empty_registry() {
    let registry = ProviderRegistry::new();
    assert!(registry.list_capabilities().is_empty());
}

#[test]
fn test_provider_count_multiple() {
    let mut registry = ProviderRegistry::new();
    assert_eq!(registry.provider_count(), 0);
    registry
        .register(security_provider_with_id(
            "p1",
            vec![SecurityFeature::Encryption],
            TrustLevel::High,
        ))
        .unwrap();
    registry
        .register(security_provider_with_id(
            "p2",
            vec![SecurityFeature::Signing],
            TrustLevel::High,
        ))
        .unwrap();
    assert_eq!(registry.provider_count(), 2);
}

#[test]
fn test_clear_resets_list_and_count() {
    let mut registry = ProviderRegistry::new();
    registry.register(create_test_security_provider()).unwrap();
    registry.clear();
    assert_eq!(registry.provider_count(), 0);
    assert!(registry.list_capabilities().is_empty());
}

#[test]
fn test_degraded_provider_excluded_from_find_best_match() {
    let mut registry = ProviderRegistry::new();
    let mut p = create_test_security_provider();
    p.health = HealthStatus::Degraded;
    registry.register(p).unwrap();

    let request = CapabilityType::Security {
        features: vec![SecurityFeature::Encryption],
        min_trust_level: TrustLevel::Medium,
    };
    assert!(registry.find_best_match(&request).unwrap().is_none());
}
