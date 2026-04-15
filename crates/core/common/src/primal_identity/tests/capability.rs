// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::*;

#[test]
fn test_capability_equality() {
    let cap1 = Capability::Compute(ComputeCapability::NativeExecution);
    let cap2 = Capability::Compute(ComputeCapability::NativeExecution);
    let cap3 = Capability::Compute(ComputeCapability::WasmExecution);

    assert_eq!(cap1, cap2);
    assert_ne!(cap1, cap3);
}

#[test]
fn test_capability_custom() {
    let cap1 = Capability::Custom {
        name: "custom-service".to_string(),
        version: "1.0".to_string(),
    };
    let cap2 = Capability::Custom {
        name: "custom-service".to_string(),
        version: "1.0".to_string(),
    };

    assert_eq!(cap1, cap2);
}

#[test]
fn test_coordination_capability_default() {
    let cap = CoordinationCapability::default();
    assert_eq!(cap, CoordinationCapability::ServiceDiscovery);
}

#[test]
fn test_all_compute_capabilities() {
    let caps = vec![
        ComputeCapability::NativeExecution,
        ComputeCapability::ContainerOrchestration,
        ComputeCapability::WasmExecution,
        ComputeCapability::PythonExecution,
        ComputeCapability::GpuCompute,
        ComputeCapability::EdgeExecution,
        ComputeCapability::SpecialtyHardware,
    ];

    assert_eq!(caps.len(), 7);
}

#[test]
fn test_all_storage_capabilities() {
    let caps = vec![
        StorageCapability::ObjectStorage,
        StorageCapability::BlockStorage,
        StorageCapability::FileStorage,
        StorageCapability::Database,
        StorageCapability::Cache,
        StorageCapability::ArtifactStorage,
    ];

    assert_eq!(caps.len(), 6);
}

#[test]
fn test_all_auth_capabilities() {
    let caps = vec![
        AuthCapability::UserAuth,
        AuthCapability::ServiceAuth,
        AuthCapability::TokenManagement,
        AuthCapability::OAuthProvider,
        AuthCapability::SamlProvider,
    ];

    assert_eq!(caps.len(), 5);
}

#[test]
fn test_all_crypto_capabilities() {
    let caps = vec![
        CryptoCapability::Encryption,
        CryptoCapability::KeyManagement,
        CryptoCapability::CertificateAuthority,
        CryptoCapability::SecretsManagement,
        CryptoCapability::HardwareSecurity,
        CryptoCapability::GeneticEntropy,
        CryptoCapability::DigitalSignatures,
        CryptoCapability::Hashing,
    ];

    assert_eq!(caps.len(), 8);
}

#[test]
fn test_all_coordination_capabilities() {
    let caps = vec![
        CoordinationCapability::ServiceDiscovery,
        CoordinationCapability::LoadBalancing,
        CoordinationCapability::HealthChecking,
        CoordinationCapability::ConfigManagement,
        CoordinationCapability::WorkflowOrchestration,
    ];

    assert_eq!(caps.len(), 5);
}

#[test]
fn test_all_discovery_capabilities() {
    let caps = vec![
        DiscoveryCapability::CapabilityDiscovery,
        DiscoveryCapability::DnsDiscovery,
        DiscoveryCapability::MdnsDiscovery,
        DiscoveryCapability::RegistryDiscovery,
    ];

    assert_eq!(caps.len(), 4);
}

#[test]
fn test_capability_debug_formatting() {
    let cap = Capability::Compute(ComputeCapability::WasmExecution);
    let debug_str = format!("{cap:?}");
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("Compute"));
    assert!(debug_str.contains("WasmExecution"));

    let custom = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let custom_debug = format!("{custom:?}");
    assert!(!custom_debug.is_empty());
    assert!(custom_debug.contains("Custom"));
}

#[test]
fn test_capability_compute_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cap1 = Capability::Compute(ComputeCapability::WasmExecution);
    let cap2 = Capability::Compute(ComputeCapability::WasmExecution);
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    cap1.hash(&mut h1);
    cap2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_coordination_capability_all_variants() {
    let _ = CoordinationCapability::ServiceDiscovery;
    let _ = CoordinationCapability::LoadBalancing;
    let _ = CoordinationCapability::HealthChecking;
    let _ = CoordinationCapability::ConfigManagement;
    let _ = CoordinationCapability::WorkflowOrchestration;
}

#[test]
fn test_capability_coordination_default() {
    let default = CoordinationCapability::default();
    assert_eq!(default, CoordinationCapability::ServiceDiscovery);
}
