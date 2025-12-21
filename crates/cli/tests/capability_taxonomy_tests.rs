//! Capability Taxonomy Tests
//!
//! Comprehensive tests for the capability-based discovery system.
//! Validates capability identifiers, matching, and standard capabilities.

use toadstool_cli::ecosystem::capabilities::taxonomy::{CapabilityId, StandardCapability};

// ===== CAPABILITY ID TESTS =====

#[test]
fn test_capability_id_creation() {
    let cap = CapabilityId::new("crypto.signature.ed25519");
    assert_eq!(cap.as_str(), "crypto.signature.ed25519");
}

#[test]
fn test_capability_id_from_string() {
    let cap: CapabilityId = "storage.object.s3".to_string().into();
    assert_eq!(cap.as_str(), "storage.object.s3");
}

#[test]
fn test_capability_id_from_str() {
    let cap: CapabilityId = "compute.wasm.component-model".into();
    assert_eq!(cap.as_str(), "compute.wasm.component-model");
}

#[test]
fn test_capability_id_category() {
    let cap = CapabilityId::new("crypto.signature.ed25519");
    assert_eq!(cap.category(), "crypto");
}

#[test]
fn test_capability_id_subcategory() {
    let cap = CapabilityId::new("crypto.signature.ed25519");
    assert_eq!(cap.subcategory(), Some("signature"));
}

#[test]
fn test_capability_id_subcategory_none() {
    let cap = CapabilityId::new("compute");
    assert_eq!(cap.subcategory(), None);
}

#[test]
fn test_capability_id_exact_match() {
    let cap = CapabilityId::new("crypto.signature.ed25519");
    assert!(cap.matches("crypto.signature.ed25519"));
    assert!(!cap.matches("crypto.signature.ecdsa"));
}

#[test]
fn test_capability_id_wildcard_match() {
    let cap = CapabilityId::new("crypto.signature.ed25519");

    // Wildcard matching
    assert!(cap.matches("crypto.signature.*"));
    assert!(cap.matches("crypto.*"));
    assert!(!cap.matches("storage.*"));
}

#[test]
fn test_capability_id_display() {
    let cap = CapabilityId::new("networking.http");
    assert_eq!(format!("{}", cap), "networking.http");
}

#[test]
fn test_capability_id_clone() {
    let cap1 = CapabilityId::new("compute.gpu");
    let cap2 = cap1.clone();
    assert_eq!(cap1.as_str(), cap2.as_str());
}

#[test]
fn test_capability_id_equality() {
    let cap1 = CapabilityId::new("monitoring.metrics");
    let cap2 = CapabilityId::new("monitoring.metrics");
    let cap3 = CapabilityId::new("monitoring.logs");

    assert_eq!(cap1, cap2);
    assert_ne!(cap1, cap3);
}

#[test]
fn test_capability_id_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(CapabilityId::new("crypto.signature.ed25519"));
    set.insert(CapabilityId::new("crypto.signature.ecdsa"));
    set.insert(CapabilityId::new("crypto.signature.ed25519")); // Duplicate

    assert_eq!(set.len(), 2);
}

// ===== STANDARD CAPABILITY TESTS =====

#[test]
fn test_standard_capability_as_str_crypto() {
    assert_eq!(
        StandardCapability::CryptoSignatureEd25519.as_str(),
        "crypto.signature.ed25519"
    );
    assert_eq!(
        StandardCapability::CryptoSignatureEcdsa.as_str(),
        "crypto.signature.ecdsa"
    );
    assert_eq!(
        StandardCapability::CryptoEncryptionAes256.as_str(),
        "crypto.encryption.aes256"
    );
}

#[test]
fn test_standard_capability_as_str_storage() {
    assert_eq!(
        StandardCapability::StorageObjectS3.as_str(),
        "storage.object.s3"
    );
    assert_eq!(StandardCapability::StorageKeyValue.as_str(), "storage.kv");
}

#[test]
fn test_standard_capability_as_str_compute() {
    assert_eq!(
        StandardCapability::ComputeWasmComponent.as_str(),
        "compute.wasm.component-model"
    );
    assert_eq!(
        StandardCapability::ComputeContainerOci.as_str(),
        "compute.container.oci"
    );
}

#[test]
fn test_standard_capability_as_str_networking() {
    assert_eq!(
        StandardCapability::NetworkingHttp.as_str(),
        "networking.http"
    );
    assert_eq!(
        StandardCapability::NetworkingGrpc.as_str(),
        "networking.grpc"
    );
}

#[test]
fn test_standard_capability_as_str_monitoring() {
    assert_eq!(
        StandardCapability::MonitoringMetrics.as_str(),
        "monitoring.metrics"
    );
    assert_eq!(
        StandardCapability::MonitoringTracing.as_str(),
        "monitoring.tracing"
    );
}

#[test]
fn test_standard_capability_id_conversion() {
    let cap = StandardCapability::CryptoSignatureEd25519;
    let id: CapabilityId = cap.into();
    assert_eq!(id.as_str(), "crypto.signature.ed25519");
}

#[test]
fn test_standard_capability_id_method() {
    let cap = StandardCapability::StorageObjectS3;
    let id = cap.id();
    assert_eq!(id.as_str(), "storage.object.s3");
}

#[test]
fn test_standard_capability_display() {
    let cap = StandardCapability::ComputeGpu;
    assert_eq!(format!("{}", cap), "compute.gpu");
}

#[test]
fn test_standard_capability_string_conversion() {
    let cap = StandardCapability::NetworkingHttp;
    let s: String = cap.into();
    assert_eq!(s, "networking.http");
}

#[test]
fn test_standard_capability_copy() {
    let cap1 = StandardCapability::CryptoRandom;
    let cap2 = cap1; // Copy, not move
    assert_eq!(cap1.as_str(), cap2.as_str());
}

#[test]
fn test_standard_capability_clone() {
    let cap1 = StandardCapability::MessagingWebsocket;
    let cap2 = cap1; // Copy, not clone
    assert_eq!(cap1.as_str(), cap2.as_str());
}

#[test]
fn test_standard_capability_equality() {
    let cap1 = StandardCapability::AuthJwt;
    let cap2 = StandardCapability::AuthJwt;
    let cap3 = StandardCapability::AuthOauth2;

    assert_eq!(cap1, cap2);
    assert_ne!(cap1, cap3);
}

// ===== CAPABILITY HIERARCHY TESTS =====

#[test]
fn test_all_crypto_capabilities_have_crypto_prefix() {
    let crypto_caps = vec![
        StandardCapability::CryptoSignatureEd25519,
        StandardCapability::CryptoSignatureEcdsa,
        StandardCapability::CryptoSignatureRsa,
        StandardCapability::CryptoEncryptionAes256,
        StandardCapability::CryptoEncryptionChaCha20,
        StandardCapability::CryptoKeyGeneration,
        StandardCapability::CryptoKeyDerivation,
        StandardCapability::CryptoRandom,
        StandardCapability::CryptoPermissionManagement,
    ];

    for cap in crypto_caps {
        assert!(
            cap.as_str().starts_with("crypto."),
            "Crypto capability '{}' should start with 'crypto.'",
            cap.as_str()
        );
    }
}

#[test]
fn test_all_storage_capabilities_have_storage_prefix() {
    let storage_caps = vec![
        StandardCapability::StorageDistributedFilesystem,
        StandardCapability::StorageObjectS3,
        StandardCapability::StorageBlock,
        StandardCapability::StorageKeyValue,
        StandardCapability::StorageDatabaseSql,
        StandardCapability::StorageDatabaseNosql,
    ];

    for cap in storage_caps {
        assert!(
            cap.as_str().starts_with("storage."),
            "Storage capability '{}' should start with 'storage.'",
            cap.as_str()
        );
    }
}

#[test]
fn test_all_compute_capabilities_have_compute_prefix() {
    let compute_caps = vec![
        StandardCapability::ComputeContainerOci,
        StandardCapability::ComputeWasmComponent,
        StandardCapability::ComputeWasmWasi,
        StandardCapability::ComputeNative,
        StandardCapability::ComputePython,
        StandardCapability::ComputeGpu,
        StandardCapability::ComputeEdge,
    ];

    for cap in compute_caps {
        assert!(
            cap.as_str().starts_with("compute."),
            "Compute capability '{}' should start with 'compute.'",
            cap.as_str()
        );
    }
}

// ===== CAPABILITY MATCHING TESTS =====

#[test]
fn test_capability_wildcard_crypto_all() {
    let capabilities = vec![
        CapabilityId::new("crypto.signature.ed25519"),
        CapabilityId::new("crypto.encryption.aes256"),
        CapabilityId::new("crypto.random"),
    ];

    let pattern = "crypto.*";
    for cap in capabilities {
        assert!(
            cap.matches(pattern),
            "Capability '{}' should match pattern '{}'",
            cap.as_str(),
            pattern
        );
    }
}

#[test]
fn test_capability_wildcard_specific_subcategory() {
    let sig_ed25519 = CapabilityId::new("crypto.signature.ed25519");
    let sig_ecdsa = CapabilityId::new("crypto.signature.ecdsa");
    let enc_aes = CapabilityId::new("crypto.encryption.aes256");

    let pattern = "crypto.signature.*";

    assert!(sig_ed25519.matches(pattern));
    assert!(sig_ecdsa.matches(pattern));
    assert!(!enc_aes.matches(pattern));
}

#[test]
fn test_capability_no_wildcard_exact_only() {
    let cap = CapabilityId::new("compute.wasm.component-model");

    assert!(cap.matches("compute.wasm.component-model"));
    assert!(!cap.matches("compute.wasm.wasi"));
    assert!(!cap.matches("compute.wasm"));
}

// ===== ZERO-COPY VALIDATION TESTS =====

#[test]
fn test_standard_capability_returns_static_str() {
    // This test validates that as_str() returns &'static str (zero-copy)
    let cap = StandardCapability::CryptoSignatureEd25519;
    let s1 = cap.as_str();
    let s2 = cap.as_str();

    // Same pointer = static string
    assert_eq!(s1.as_ptr(), s2.as_ptr());
}

#[test]
fn test_capability_strings_are_lowercase() {
    let caps = vec![
        StandardCapability::CryptoSignatureEd25519,
        StandardCapability::StorageObjectS3,
        StandardCapability::ComputeWasmComponent,
        StandardCapability::NetworkingHttp,
        StandardCapability::MonitoringMetrics,
    ];

    for cap in caps {
        let s = cap.as_str();
        assert_eq!(
            s,
            s.to_lowercase(),
            "Capability '{}' should be lowercase",
            s
        );
    }
}

#[test]
fn test_capability_strings_use_kebab_case() {
    // Verify no underscores, CamelCase, or spaces
    let caps = vec![
        StandardCapability::CryptoKeyGeneration,
        StandardCapability::CoordinationLeaderElection,
        StandardCapability::NetworkingLoadBalancer,
    ];

    for cap in caps {
        let s = cap.as_str();
        assert!(
            !s.contains('_'),
            "Capability '{}' should not contain underscores",
            s
        );
        assert!(
            !s.contains(' '),
            "Capability '{}' should not contain spaces",
            s
        );
        assert!(
            s.chars().all(|c| c.is_lowercase() || c == '.' || c == '-'),
            "Capability '{}' should use kebab-case",
            s
        );
    }
}

// ===== EDGE CASES =====

#[test]
fn test_capability_id_empty_category() {
    let cap = CapabilityId::new("");
    assert_eq!(cap.category(), "");
}

#[test]
fn test_capability_id_single_level() {
    let cap = CapabilityId::new("compute");
    assert_eq!(cap.category(), "compute");
    assert_eq!(cap.subcategory(), None);
}

#[test]
fn test_capability_wildcard_at_end_only() {
    let cap = CapabilityId::new("crypto.signature.ed25519");

    // Wildcard only works at end
    assert!(cap.matches("crypto.*"));
    assert!(cap.matches("crypto.signature.*"));

    // Not in middle (this would be a pattern bug)
    assert!(!cap.matches("cry*.signature.ed25519"));
}
