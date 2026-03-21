// SPDX-License-Identifier: AGPL-3.0-only
//! Additional tests for self-identity module
//!
//! Expanding test coverage for the self-identity system.

use toadstool::self_identity::{Capability, CapabilityRequirement, SelfIdentity};

#[test]
fn test_capability_equality() {
    let cap1 = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec!["native".to_string(), "wasm".to_string()],
        characteristics: std::collections::HashMap::new(),
    };

    let cap2 = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec!["native".to_string(), "wasm".to_string()],
        characteristics: std::collections::HashMap::new(),
    };

    assert_eq!(cap1, cap2, "Capabilities with same data should be equal");
}

#[test]
fn test_capability_inequality_different_name() {
    let cap1 = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec![],
        characteristics: std::collections::HashMap::new(),
    };

    let cap2 = Capability {
        name: "storage".to_string(),
        version: "1.0.0".to_string(),
        features: vec![],
        characteristics: std::collections::HashMap::new(),
    };

    assert_ne!(
        cap1, cap2,
        "Capabilities with different names should not be equal"
    );
}

#[test]
fn test_capability_clone() {
    let original = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec!["native".to_string()],
        characteristics: {
            let mut map = std::collections::HashMap::new();
            map.insert("performance".to_string(), "high".to_string());
            map
        },
    };

    let cloned = original.clone();

    assert_eq!(original, cloned);
    assert_eq!(cloned.name, "compute");
    assert_eq!(cloned.features.len(), 1);
    assert_eq!(cloned.characteristics.len(), 1);
}

#[test]
fn test_capability_requirement_optional() {
    let req = CapabilityRequirement {
        capability: "coordination".to_string(),
        min_version: Some("2.0.0".to_string()),
        required: false,
        features: vec![],
        purpose: "Optional service mesh integration".to_string(),
    };

    assert!(!req.required, "Requirement should be marked as optional");
    assert!(req.min_version.is_some(), "Should have version requirement");
}

#[test]
fn test_capability_requirement_required() {
    let req = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: None,
        required: true,
        features: vec!["persistence".to_string()],
        purpose: "Required for data persistence".to_string(),
    };

    assert!(req.required, "Requirement should be marked as required");
    assert!(
        req.min_version.is_none(),
        "Should not have version requirement"
    );
    assert_eq!(req.features.len(), 1);
}

#[test]
fn test_self_identity_serialization() {
    let identity = SelfIdentity::default();

    // Should serialize to JSON without errors
    let json_result = serde_json::to_string(&identity);
    assert!(json_result.is_ok(), "Should serialize to JSON successfully");

    // Should produce valid JSON output
    let json_str = json_result.unwrap();
    assert!(!json_str.is_empty(), "JSON output should not be empty");
    assert!(json_str.contains("toadstool"), "Should contain primal type");
}

#[test]
fn test_self_identity_primal_type() {
    let identity = SelfIdentity::default();
    assert_eq!(
        identity.primal_type, "toadstool",
        "Primal type should always be 'toadstool'"
    );
}

#[test]
fn test_self_identity_unique_instance_ids() {
    let id1 = SelfIdentity::default();
    let id2 = SelfIdentity::default();

    assert_ne!(
        id1.instance_id, id2.instance_id,
        "Each instance should have a unique ID"
    );
}

#[test]
fn test_capability_with_features() {
    let cap = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec![
            "native".to_string(),
            "wasm".to_string(),
            "container".to_string(),
            "gpu".to_string(),
        ],
        characteristics: std::collections::HashMap::new(),
    };

    assert_eq!(cap.features.len(), 4);
    assert!(cap.features.contains(&"native".to_string()));
    assert!(cap.features.contains(&"gpu".to_string()));
}

#[test]
fn test_capability_with_characteristics() {
    let mut characteristics = std::collections::HashMap::new();
    characteristics.insert("performance".to_string(), "high".to_string());
    characteristics.insert("latency".to_string(), "low".to_string());
    characteristics.insert("throughput".to_string(), "high".to_string());

    let cap = Capability {
        name: "compute".to_string(),
        version: "1.0.0".to_string(),
        features: vec![],
        characteristics: characteristics.clone(),
    };

    assert_eq!(cap.characteristics.len(), 3);
    assert_eq!(
        cap.characteristics.get("performance"),
        Some(&"high".to_string())
    );
    assert_eq!(cap.characteristics.get("latency"), Some(&"low".to_string()));
}

#[test]
fn test_capability_requirement_with_features() {
    let req = CapabilityRequirement {
        capability: "ai".to_string(),
        min_version: Some("3.0.0".to_string()),
        required: true,
        features: vec![
            "inference".to_string(),
            "training".to_string(),
            "gpu-acceleration".to_string(),
        ],
        purpose: "AI workload processing".to_string(),
    };

    assert_eq!(req.features.len(), 3);
    assert!(req.features.contains(&"inference".to_string()));
    assert!(req.features.contains(&"gpu-acceleration".to_string()));
    assert_eq!(req.capability, "ai");
}

#[test]
fn test_self_identity_debug_format() {
    let identity = SelfIdentity::default();
    let debug_str = format!("{identity:?}");

    assert!(!debug_str.is_empty(), "Debug format should produce output");
    assert!(
        debug_str.contains("SelfIdentity"),
        "Debug should contain struct name"
    );
}

#[test]
fn test_capability_debug_format() {
    let cap = Capability {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        features: vec![],
        characteristics: std::collections::HashMap::new(),
    };

    let debug_str = format!("{cap:?}");
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("Capability"));
}

#[test]
fn test_network_identity_protocols() {
    use toadstool::self_identity::NetworkIdentity;

    let network = NetworkIdentity {
        hostname: "localhost".to_string(),
        port: Some(8080),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec!["http".to_string(), "https".to_string(), "grpc".to_string()],
    };

    assert_eq!(network.protocols.len(), 3);
    assert!(network.protocols.contains(&"http".to_string()));
    assert!(network.protocols.contains(&"grpc".to_string()));
    assert_eq!(network.port, Some(8080));
}

#[test]
fn test_resource_profile_fields() {
    use toadstool::self_identity::ResourceProfile;

    let profile = ResourceProfile {
        cpu_cores: 8,
        memory_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
        gpu_available: true,
        storage_bytes: Some(500 * 1024 * 1024 * 1024), // 500 GB
    };

    assert_eq!(profile.cpu_cores, 8);
    assert_eq!(profile.memory_bytes, 16 * 1024 * 1024 * 1024);
    assert!(profile.gpu_available);
    assert!(profile.storage_bytes.is_some());
}
