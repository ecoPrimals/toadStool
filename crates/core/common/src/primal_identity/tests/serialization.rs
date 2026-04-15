// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use super::super::*;

#[test]
fn test_capability_serialize_deserialize() {
    let cap = Capability::Compute(ComputeCapability::GpuCompute);
    let json = serde_json::to_string(&cap).unwrap();
    let deserialized: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(cap, deserialized);

    let custom = Capability::Custom {
        name: "custom-service".to_string(),
        version: "2.0".to_string(),
    };
    let json = serde_json::to_string(&custom).unwrap();
    let deserialized: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(custom, deserialized);
}

#[test]
fn test_discovered_service_serialize_deserialize() {
    let service = DiscoveredService {
        id: Some("svc-1".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        healthy: true,
        metadata: {
            let mut m = HashMap::new();
            m.insert("version".into(), "1.0".into());
            m
        },
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(service.id, restored.id);
    assert_eq!(service.healthy, restored.healthy);
    assert_eq!(service.capabilities.len(), restored.capabilities.len());
}

#[test]
fn test_discovered_service_serde_roundtrip_empty() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![],
        healthy: false,
        metadata: HashMap::new(),
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(service.id, restored.id);
    assert_eq!(service.healthy, restored.healthy);
    assert!(restored.capabilities.is_empty());
}

#[test]
fn test_capability_compute_edge_execution() {
    let cap = Capability::Compute(ComputeCapability::EdgeExecution);
    let json = serde_json::to_string(&cap).expect("serialize");
    let restored: Capability = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(
        restored,
        Capability::Compute(ComputeCapability::EdgeExecution)
    ));
}

#[test]
fn test_capability_compute_specialty_hardware() {
    let cap = Capability::Compute(ComputeCapability::SpecialtyHardware);
    let json = serde_json::to_string(&cap).expect("serialize");
    let restored: Capability = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(
        restored,
        Capability::Compute(ComputeCapability::SpecialtyHardware)
    ));
}

#[test]
fn test_capability_storage_all_variants_serde() {
    let variants = [
        StorageCapability::BlockStorage,
        StorageCapability::FileStorage,
        StorageCapability::Database,
        StorageCapability::Cache,
        StorageCapability::ArtifactStorage,
    ];
    for cap in &variants {
        let cap_enum = Capability::Storage(cap.clone());
        let json = serde_json::to_string(&cap_enum).expect("serialize");
        let restored: Capability = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&cap_enum, &restored);
    }
}

#[test]
fn test_discovered_service_metadata_roundtrip() {
    let mut meta = HashMap::new();
    meta.insert("region".to_string(), "us-east".to_string());
    meta.insert("version".to_string(), "2.0".to_string());
    let service = DiscoveredService {
        id: Some("m".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![ServiceEndpoint::http("x", 80)],
        healthy: true,
        metadata: meta.clone(),
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.metadata.get("region"),
        Some(&"us-east".to_string())
    );
    assert_eq!(restored.metadata.get("version"), Some(&"2.0".to_string()));
}
