// SPDX-License-Identifier: AGPL-3.0-or-later
//! Expanded tests for `self_identity` module
//!
//! Coverage expansion: `self_identity.rs` had minimal test coverage
//! Adding comprehensive tests for all paths and edge cases

use std::collections::HashMap;
use toadstool::self_identity::*;
use uuid::Uuid;

/// Test resource detection returns valid values
#[test]
fn test_resource_detection() {
    let identity = SelfIdentity::new();

    // CPU cores should be at least 1
    assert!(identity.resources.cpu_cores >= 1);

    // Memory should be non-zero (system has some memory)
    assert!(identity.resources.memory_bytes > 0);

    // GPU detection should return a valid boolean (always true or false)
    let _ = identity.resources.gpu_available; // type-checked, always valid
}

/// Test instance ID uniqueness
#[test]
fn test_instance_id_uniqueness() {
    let id1 = SelfIdentity::new();
    let id2 = SelfIdentity::new();

    // Each instance should have unique ID
    assert_ne!(id1.instance_id, id2.instance_id);
}

/// Test primal type is always toadstool
#[test]
fn test_primal_type_constant() {
    let identity = SelfIdentity::new();
    assert_eq!(identity.primal_type, "toadstool");
}

/// Test version comes from Cargo.toml
#[test]
fn test_version_from_cargo() {
    let identity = SelfIdentity::new();
    assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
    assert!(!identity.version.is_empty());
}

/// Test capabilities structure
#[test]
fn test_capabilities_structure() {
    let identity = SelfIdentity::new();

    // Should have multiple capabilities
    assert!(identity.capabilities.len() >= 3);

    // Each capability should have required fields
    for cap in &identity.capabilities {
        assert!(!cap.name.is_empty());
        assert!(!cap.version.is_empty());
    }
}

/// Test compute capability details
#[test]
fn test_compute_capability() {
    let identity = SelfIdentity::new();

    let compute = identity
        .capabilities
        .iter()
        .find(|c| c.name == "compute")
        .expect("Compute capability should exist");

    assert_eq!(compute.version, "1.0");
    assert!(compute.features.contains(&"cpu".to_string()));
    assert!(compute.features.contains(&"parallel".to_string()));
    assert!(compute.characteristics.contains_key("type"));
}

/// Test orchestration capability
#[test]
fn test_orchestration_capability() {
    let identity = SelfIdentity::new();

    let orch = identity
        .capabilities
        .iter()
        .find(|c| c.name == "orchestration")
        .expect("Orchestration capability should exist");

    assert_eq!(orch.version, "1.0");
    assert!(orch.features.contains(&"workload-management".to_string()));
    assert!(orch.features.contains(&"resource-allocation".to_string()));
    assert!(orch.features.contains(&"auto-scheduling".to_string()));
}

/// Test BYOB capability
#[test]
fn test_byob_capability() {
    let identity = SelfIdentity::new();

    let byob = identity
        .capabilities
        .iter()
        .find(|c| c.name == "byob")
        .expect("BYOB capability should exist");

    assert_eq!(byob.version, "1.0");
    assert!(byob.features.contains(&"deployment".to_string()));
    assert!(byob.features.contains(&"lifecycle".to_string()));
}

/// Test requirements structure
#[test]
fn test_requirements_structure() {
    let identity = SelfIdentity::new();

    // Should have multiple requirements
    assert!(identity.requirements.len() >= 4);

    // Each requirement should have required fields
    for req in &identity.requirements {
        assert!(!req.capability.is_empty());
        assert!(!req.purpose.is_empty());
    }
}

/// Test coordination requirement
#[test]
fn test_coordination_requirement() {
    let identity = SelfIdentity::new();

    let coord = identity
        .requirements
        .iter()
        .find(|r| r.capability == "coordination")
        .expect("Coordination requirement should exist");

    assert!(!coord.required); // Optional
    assert!(coord.features.contains(&"routing".to_string()));
    assert!(coord.features.contains(&"discovery".to_string()));
}

/// Test storage requirement
#[test]
fn test_storage_requirement() {
    let identity = SelfIdentity::new();

    let storage = identity
        .requirements
        .iter()
        .find(|r| r.capability == "storage")
        .expect("Storage requirement should exist");

    assert!(!storage.required); // Optional - we can use local
    assert!(storage.features.contains(&"object-store".to_string()));
}

/// Test security requirement
#[test]
fn test_security_requirement() {
    let identity = SelfIdentity::new();

    let security = identity
        .requirements
        .iter()
        .find(|r| r.capability == "security")
        .expect("Security requirement should exist");

    assert!(!security.required); // Optional - we have basic security
    assert!(security.features.contains(&"authentication".to_string()));
}

/// Test AI requirement
#[test]
fn test_ai_requirement() {
    let identity = SelfIdentity::new();

    let ai = identity
        .requirements
        .iter()
        .find(|r| r.capability == "ai")
        .expect("AI requirement should exist");

    assert!(!ai.required); // Optional
    assert!(ai.features.contains(&"orchestration".to_string()));
}

/// Test network identity with port
#[test]
fn test_network_with_port() {
    let identity = SelfIdentity::new().with_network(
        "example.com".to_string(),
        Some(9090),
        vec!["http".to_string(), "grpc".to_string()],
    );

    assert!(identity.network.is_some());
    let network = identity.network.unwrap();
    assert_eq!(network.hostname, "example.com");
    assert_eq!(network.port, Some(9090));
    assert_eq!(network.endpoint, "example.com:9090");
    assert_eq!(network.protocols.len(), 2);
}

/// Test network identity without port
#[test]
fn test_network_without_port() {
    let identity =
        SelfIdentity::new().with_network("example.com".to_string(), None, vec!["unix".to_string()]);

    assert!(identity.network.is_some());
    let network = identity.network.unwrap();
    assert_eq!(network.hostname, "example.com");
    assert_eq!(network.port, None);
    assert_eq!(network.endpoint, "example.com"); // No port in endpoint
}

/// Test network with empty protocols
#[test]
fn test_network_empty_protocols() {
    let identity = SelfIdentity::new().with_network("localhost".to_string(), Some(8080), vec![]);

    assert!(identity.network.is_some());
    let network = identity.network.unwrap();
    assert!(network.protocols.is_empty());
}

/// Test advertisement without network
#[test]
fn test_advertisement_no_network() {
    let identity = SelfIdentity::new();
    let ad = identity.to_advertisement();

    assert_eq!(ad.primal_type, "toadstool");
    assert!(ad.endpoint.is_none());
    assert!(ad.protocols.is_empty());
    assert!(!ad.capabilities.is_empty());
}

/// Test advertisement with network
#[test]
fn test_advertisement_with_network() {
    let identity = SelfIdentity::new().with_network(
        "server.local".to_string(),
        Some(7777),
        vec!["http".to_string(), "websocket".to_string()],
    );

    let ad = identity.to_advertisement();

    assert_eq!(ad.primal_type, "toadstool");
    assert_eq!(ad.endpoint, Some("server.local:7777".to_string()));
    assert_eq!(ad.protocols.len(), 2);
    assert!(ad.protocols.contains(&"http".to_string()));
    assert!(ad.protocols.contains(&"websocket".to_string()));
}

/// Test advertisement preserves capabilities
#[test]
fn test_advertisement_capabilities() {
    let identity = SelfIdentity::new();
    let ad = identity.to_advertisement();

    // Advertisement should have same capabilities as identity
    assert_eq!(ad.capabilities.len(), identity.capabilities.len());
    for (ad_cap, id_cap) in ad.capabilities.iter().zip(identity.capabilities.iter()) {
        assert_eq!(ad_cap.name, id_cap.name);
        assert_eq!(ad_cap.version, id_cap.version);
    }
}

/// Test `matches_requirement` with exact match
#[test]
fn test_matches_requirement_exact() {
    let identity = SelfIdentity::new();

    let requirement = CapabilityRequirement {
        capability: "compute".to_string(),
        min_version: Some("1.0".to_string()),
        required: false,
        features: vec!["cpu".to_string()],
        purpose: "Test".to_string(),
    };

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "toadstool".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability {
            name: "compute".to_string(),
            version: "1.0".to_string(),
            features: vec!["cpu".to_string(), "parallel".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8080".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(identity.matches_requirement(&requirement, &service));
}

/// Test `matches_requirement` with missing feature
#[test]
fn test_matches_requirement_missing_feature() {
    let identity = SelfIdentity::new();

    let requirement = CapabilityRequirement {
        capability: "compute".to_string(),
        min_version: Some("1.0".to_string()),
        required: false,
        features: vec!["gpu".to_string()], // Service doesn't have this
        purpose: "Test".to_string(),
    };

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "toadstool".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability {
            name: "compute".to_string(),
            version: "1.0".to_string(),
            features: vec!["cpu".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8080".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(!identity.matches_requirement(&requirement, &service));
}

/// Test `matches_requirement` with wrong capability
#[test]
fn test_matches_requirement_wrong_capability() {
    let identity = SelfIdentity::new();

    let requirement = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: Some("1.0".to_string()),
        required: false,
        features: vec![],
        purpose: "Test".to_string(),
    };

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "toadstool".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability {
            name: "compute".to_string(), // Wrong capability
            version: "1.0".to_string(),
            features: vec![],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8080".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(!identity.matches_requirement(&requirement, &service));
}

/// Test `matches_requirement` with empty features
#[test]
fn test_matches_requirement_empty_features() {
    let identity = SelfIdentity::new();

    let requirement = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: Some("1.0".to_string()),
        required: false,
        features: vec![], // No specific features required
        purpose: "Test".to_string(),
    };

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "nestgate".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "1.0".to_string(),
            features: vec!["object-store".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8082".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    // Should match because no specific features required
    assert!(identity.matches_requirement(&requirement, &service));
}

/// Test discovered service from advertisement
#[test]
fn test_discovered_service_from_advertisement() {
    let identity = SelfIdentity::new().with_network(
        "peer.local".to_string(),
        Some(5555),
        vec!["grpc".to_string()],
    );

    let ad = identity.to_advertisement();
    let service: DiscoveredService = ad.into();

    assert_eq!(service.primal_type, "toadstool");
    assert_eq!(service.endpoint, "peer.local:5555");
    assert!(!service.capabilities.is_empty());
}

/// Test discovered service from advertisement without endpoint
#[test]
fn test_discovered_service_no_endpoint() {
    let identity = SelfIdentity::new();
    let ad = identity.to_advertisement();
    let service: DiscoveredService = ad.into();

    // Should have fallback endpoint
    assert_eq!(service.endpoint, "unknown");
}

/// Test discovered service timestamps
#[test]
fn test_discovered_service_timestamps() {
    let identity = SelfIdentity::new();
    let ad = identity.to_advertisement();
    let service: DiscoveredService = ad.into();

    // discovered_at and last_seen should be set
    assert_eq!(service.discovered_at, service.last_seen);
}

/// Test capability equality
#[test]
fn test_capability_equality() {
    let cap1 = Capability {
        name: "test".to_string(),
        version: "1.0".to_string(),
        features: vec!["feat1".to_string()],
        characteristics: HashMap::new(),
    };

    let cap2 = Capability {
        name: "test".to_string(),
        version: "1.0".to_string(),
        features: vec!["feat1".to_string()],
        characteristics: HashMap::new(),
    };

    assert_eq!(cap1, cap2);
}

/// Test capability inequality
#[test]
fn test_capability_inequality() {
    let cap1 = Capability {
        name: "test".to_string(),
        version: "1.0".to_string(),
        features: vec![],
        characteristics: HashMap::new(),
    };

    let cap2 = Capability {
        name: "different".to_string(),
        version: "1.0".to_string(),
        features: vec![],
        characteristics: HashMap::new(),
    };

    assert_ne!(cap1, cap2);
}

/// Test default implementation
#[test]
fn test_default_implementation() {
    let identity1 = SelfIdentity::new();
    let identity2 = SelfIdentity::default();

    // Both should create valid identities (though with different instance IDs)
    assert_eq!(identity1.primal_type, identity2.primal_type);
    assert_eq!(identity1.version, identity2.version);
    assert_eq!(identity1.capabilities.len(), identity2.capabilities.len());
}

/// Test serialization of `SelfIdentity`
#[test]
fn test_self_identity_serialization() {
    let identity = SelfIdentity::new().with_network(
        "test.local".to_string(),
        Some(3000),
        vec!["http".to_string()],
    );

    // Just test that it can serialize (deserialization has lifetime issues with &'static str)
    let json = serde_json::to_string(&identity).expect("Should serialize");
    assert!(json.contains("toadstool"));
    assert!(json.contains("test.local"));
}

/// Test serialization of `ServiceAdvertisement`
#[test]
fn test_advertisement_serialization() {
    let identity = SelfIdentity::new();
    let ad = identity.to_advertisement();

    let json = serde_json::to_string(&ad).expect("Should serialize");
    assert!(json.contains("toadstool"));
    assert!(!json.is_empty());
}

/// Test serialization of `DiscoveredService`
#[test]
fn test_discovered_service_serialization() {
    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    let json = serde_json::to_string(&service).expect("Should serialize");
    assert!(json.contains("localhost:8080"));
    assert!(!json.is_empty());
}

/// Test resource profile values are reasonable
#[test]
fn test_resource_profile_reasonable() {
    let identity = SelfIdentity::new();
    let resources = &identity.resources;

    // CPU cores should be reasonable (1-256 typical range)
    assert!(resources.cpu_cores >= 1);
    assert!(resources.cpu_cores <= 256);

    // Memory should be reasonable (at least 100MB, less than 1TB)
    assert!(resources.memory_bytes >= 100_000_000); // 100 MB
    assert!(resources.memory_bytes <= 1_099_511_627_776); // 1 TB

    // Storage is optional
    assert!(resources.storage_bytes.is_none() || resources.storage_bytes.unwrap() > 0);
}

/// Test cloning `SelfIdentity`
#[test]
fn test_self_identity_clone() {
    let identity1 = SelfIdentity::new();
    let identity2 = identity1.clone();

    assert_eq!(identity1.instance_id, identity2.instance_id);
    assert_eq!(identity1.primal_type, identity2.primal_type);
    assert_eq!(identity1.version, identity2.version);
}

/// Test debug formatting
#[test]
fn test_debug_format() {
    let identity = SelfIdentity::new();
    let debug_str = format!("{identity:?}");

    assert!(debug_str.contains("SelfIdentity"));
    assert!(debug_str.contains("toadstool"));
}
