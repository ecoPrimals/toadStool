//! Comprehensive tests for NestGate integration types

use std::collections::HashMap;
use toadstool_integration_nestgate::*;
use uuid::Uuid;

// ============================================================================
// StorageTier Tests
// ============================================================================

#[test]
fn test_storage_tier_hot() {
    let tier = StorageTier::Hot;
    let json = serde_json::to_string(&tier).unwrap();
    let deserialized: StorageTier = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, StorageTier::Hot));
}

#[test]
fn test_storage_tier_standard() {
    let tier = StorageTier::Standard;
    let json = serde_json::to_string(&tier).unwrap();
    let deserialized: StorageTier = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, StorageTier::Standard));
}

#[test]
fn test_storage_tier_cold() {
    let tier = StorageTier::Cold;
    assert!(matches!(tier, StorageTier::Cold));
}

#[test]
fn test_storage_tier_archive() {
    let tier = StorageTier::Archive;
    assert!(matches!(tier, StorageTier::Archive));
}

#[test]
fn test_storage_tier_clone() {
    let tier = StorageTier::Hot;
    let cloned = tier.clone();

    assert!(matches!(cloned, StorageTier::Hot));
}

// ============================================================================
// CompressionType Tests
// ============================================================================

#[test]
fn test_compression_none() {
    let compression = CompressionType::None;
    assert!(matches!(compression, CompressionType::None));
}

#[test]
fn test_compression_auto() {
    let compression = CompressionType::Auto;
    assert!(matches!(compression, CompressionType::Auto));
}

#[test]
fn test_compression_gzip() {
    let compression = CompressionType::Gzip;
    assert!(matches!(compression, CompressionType::Gzip));
}

#[test]
fn test_compression_lz4() {
    let compression = CompressionType::Lz4;
    assert!(matches!(compression, CompressionType::Lz4));
}

#[test]
fn test_compression_zstd() {
    let compression = CompressionType::Zstd;
    assert!(matches!(compression, CompressionType::Zstd));
}

#[test]
fn test_compression_serialization() {
    let compression = CompressionType::Gzip;
    let json = serde_json::to_string(&compression).unwrap();
    let deserialized: CompressionType = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, CompressionType::Gzip));
}

// ============================================================================
// EncryptionType Tests
// ============================================================================

#[test]
fn test_encryption_none() {
    let encryption = EncryptionType::None;
    assert!(matches!(encryption, EncryptionType::None));
}

#[test]
fn test_encryption_default() {
    let encryption = EncryptionType::Default;
    assert!(matches!(encryption, EncryptionType::Default));
}

#[test]
fn test_encryption_aes256() {
    let encryption = EncryptionType::Aes256;
    assert!(matches!(encryption, EncryptionType::Aes256));
}

#[test]
fn test_encryption_custom() {
    let encryption = EncryptionType::Custom("chacha20".to_string());

    match encryption {
        EncryptionType::Custom(algo) => assert_eq!(algo, "chacha20"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_encryption_serialization() {
    let encryption = EncryptionType::Aes256;
    let json = serde_json::to_string(&encryption).unwrap();
    let deserialized: EncryptionType = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, EncryptionType::Aes256));
}

// ============================================================================
// ArtifactType Tests
// ============================================================================

#[test]
fn test_artifact_type_execution_input() {
    let artifact_type = ArtifactType::ExecutionInput;
    assert!(matches!(artifact_type, ArtifactType::ExecutionInput));
}

#[test]
fn test_artifact_type_execution_output() {
    let artifact_type = ArtifactType::ExecutionOutput;
    assert!(matches!(artifact_type, ArtifactType::ExecutionOutput));
}

#[test]
fn test_artifact_type_logs() {
    let artifact_type = ArtifactType::Logs;
    assert!(matches!(artifact_type, ArtifactType::Logs));
}

#[test]
fn test_artifact_type_binary() {
    let artifact_type = ArtifactType::Binary;
    assert!(matches!(artifact_type, ArtifactType::Binary));
}

#[test]
fn test_artifact_type_container_image() {
    let artifact_type = ArtifactType::ContainerImage;
    assert!(matches!(artifact_type, ArtifactType::ContainerImage));
}

#[test]
fn test_artifact_type_wasm_module() {
    let artifact_type = ArtifactType::WasmModule;
    assert!(matches!(artifact_type, ArtifactType::WasmModule));
}

#[test]
fn test_artifact_type_data_file() {
    let artifact_type = ArtifactType::DataFile;
    assert!(matches!(artifact_type, ArtifactType::DataFile));
}

#[test]
fn test_artifact_type_configuration() {
    let artifact_type = ArtifactType::Configuration;
    assert!(matches!(artifact_type, ArtifactType::Configuration));
}

#[test]
fn test_artifact_type_model() {
    let artifact_type = ArtifactType::Model;
    assert!(matches!(artifact_type, ArtifactType::Model));
}

#[test]
fn test_artifact_type_custom() {
    let artifact_type = ArtifactType::Custom("custom-data".to_string());

    match artifact_type {
        ArtifactType::Custom(name) => assert_eq!(name, "custom-data"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_artifact_type_serialization() {
    let artifact_type = ArtifactType::Model;
    let json = serde_json::to_string(&artifact_type).unwrap();
    let deserialized: ArtifactType = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, ArtifactType::Model));
}

// ============================================================================
// NestGateError Tests
// ============================================================================

#[test]
fn test_nestgate_error_connection() {
    let error = NestGateError::Connection("timeout".to_string());
    assert!(error.to_string().contains("Connection"));
    assert!(error.to_string().contains("timeout"));
}

#[test]
fn test_nestgate_error_authentication() {
    let error = NestGateError::Authentication("invalid token".to_string());
    assert!(error.to_string().contains("Authentication"));
}

#[test]
fn test_nestgate_error_storage() {
    let error = NestGateError::Storage("disk full".to_string());
    assert!(error.to_string().contains("Storage"));
}

#[test]
fn test_nestgate_error_pipeline() {
    let error = NestGateError::Pipeline("execution failed".to_string());
    assert!(error.to_string().contains("pipeline"));
}

#[test]
fn test_nestgate_error_versioning() {
    let error = NestGateError::Versioning("conflict".to_string());
    assert!(error.to_string().contains("Versioning"));
}

#[test]
fn test_nestgate_error_network() {
    let error = NestGateError::Network("unreachable".to_string());
    assert!(error.to_string().contains("Network"));
}

#[test]
fn test_nestgate_error_internal() {
    let error = NestGateError::Internal("unexpected state".to_string());
    assert!(error.to_string().contains("Internal"));
}

// ============================================================================
// StorageInfo Tests
// ============================================================================

#[test]
fn test_storage_info_creation() {
    let storage_info = StorageInfo {
        node_id: "node-123".to_string(),
        path: "/data/artifacts/file1".to_string(),
        tier: StorageTier::Hot,
        replicated: true,
        compression: CompressionType::Gzip,
        encryption: EncryptionType::Aes256,
    };

    assert_eq!(storage_info.node_id, "node-123");
    assert_eq!(storage_info.path, "/data/artifacts/file1");
    assert!(storage_info.replicated);
}

#[test]
fn test_storage_info_no_replication() {
    let storage_info = StorageInfo {
        node_id: "node-single".to_string(),
        path: "/tmp/data".to_string(),
        tier: StorageTier::Standard,
        replicated: false,
        compression: CompressionType::None,
        encryption: EncryptionType::None,
    };

    assert!(!storage_info.replicated);
}

#[test]
fn test_storage_info_serialization() {
    let storage_info = StorageInfo {
        node_id: "test-node".to_string(),
        path: "/test/path".to_string(),
        tier: StorageTier::Cold,
        replicated: true,
        compression: CompressionType::Zstd,
        encryption: EncryptionType::Default,
    };

    let json = serde_json::to_string(&storage_info).unwrap();
    let deserialized: StorageInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.node_id, "test-node");
}

// ============================================================================
// ArtifactMetadata Tests
// ============================================================================

#[test]
fn test_artifact_metadata_minimal() {
    let metadata = ArtifactMetadata {
        id: "artifact-1".to_string(),
        artifact_type: ArtifactType::Binary,
        content_type: "application/octet-stream".to_string(),
        size_bytes: 1024,
        checksum: "sha256:abc123".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: None,
        tags: HashMap::new(),
        execution_id: None,
        storage_info: StorageInfo {
            node_id: "node1".to_string(),
            path: "/data".to_string(),
            tier: StorageTier::Standard,
            replicated: false,
            compression: CompressionType::None,
            encryption: EncryptionType::None,
        },
        version: None,
    };

    assert_eq!(metadata.id, "artifact-1");
    assert_eq!(metadata.size_bytes, 1024);
    assert!(metadata.tags.is_empty());
}

#[test]
fn test_artifact_metadata_with_tags() {
    let mut tags = HashMap::new();
    tags.insert("env".to_string(), "production".to_string());
    tags.insert("version".to_string(), "1.0.0".to_string());

    let metadata = ArtifactMetadata {
        id: "artifact-2".to_string(),
        artifact_type: ArtifactType::Model,
        content_type: "application/x-tensorflow".to_string(),
        size_bytes: 10240,
        checksum: "sha256:def456".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: Some(std::time::SystemTime::now()),
        tags: tags.clone(),
        execution_id: Some(Uuid::new_v4()),
        storage_info: StorageInfo {
            node_id: "node2".to_string(),
            path: "/models".to_string(),
            tier: StorageTier::Hot,
            replicated: true,
            compression: CompressionType::Lz4,
            encryption: EncryptionType::Aes256,
        },
        version: None,
    };

    assert_eq!(metadata.tags.len(), 2);
    assert_eq!(metadata.tags.get("env").unwrap(), "production");
}

#[test]
fn test_artifact_metadata_large_size() {
    let metadata = ArtifactMetadata {
        id: "large-artifact".to_string(),
        artifact_type: ArtifactType::DataFile,
        content_type: "application/json".to_string(),
        size_bytes: 10_737_418_240, // 10 GB
        checksum: "sha256:large".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: None,
        tags: HashMap::new(),
        execution_id: None,
        storage_info: StorageInfo {
            node_id: "storage-node".to_string(),
            path: "/bigdata".to_string(),
            tier: StorageTier::Cold,
            replicated: true,
            compression: CompressionType::Zstd,
            encryption: EncryptionType::Aes256,
        },
        version: None,
    };

    assert_eq!(metadata.size_bytes, 10_737_418_240);
}

#[test]
fn test_artifact_metadata_serialization() {
    let metadata = ArtifactMetadata {
        id: "test".to_string(),
        artifact_type: ArtifactType::Logs,
        content_type: "text/plain".to_string(),
        size_bytes: 512,
        checksum: "sha256:test".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: None,
        tags: HashMap::new(),
        execution_id: None,
        storage_info: StorageInfo {
            node_id: "node".to_string(),
            path: "/logs".to_string(),
            tier: StorageTier::Standard,
            replicated: false,
            compression: CompressionType::Auto,
            encryption: EncryptionType::Default,
        },
        version: None,
    };

    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test");
    assert_eq!(deserialized.size_bytes, 512);
}

// ============================================================================
// Integration Scenarios
// ============================================================================

#[test]
fn test_scenario_ml_model_storage() {
    let metadata = ArtifactMetadata {
        id: "ml-model-v1.0".to_string(),
        artifact_type: ArtifactType::Model,
        content_type: "application/x-pytorch".to_string(),
        size_bytes: 524_288_000, // 500 MB
        checksum: "sha256:model123".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: None,
        tags: {
            let mut tags = HashMap::new();
            tags.insert("model_type".to_string(), "transformer".to_string());
            tags.insert("version".to_string(), "1.0".to_string());
            tags
        },
        execution_id: Some(Uuid::new_v4()),
        storage_info: StorageInfo {
            node_id: "ml-storage-1".to_string(),
            path: "/models/transformer".to_string(),
            tier: StorageTier::Hot,
            replicated: true,
            compression: CompressionType::Lz4,
            encryption: EncryptionType::Aes256,
        },
        version: None,
    };

    assert!(matches!(metadata.artifact_type, ArtifactType::Model));
    assert!(matches!(metadata.storage_info.tier, StorageTier::Hot));
    assert!(metadata.storage_info.replicated);
}

#[test]
fn test_scenario_log_archival() {
    let metadata = ArtifactMetadata {
        id: "logs-2025-10-14".to_string(),
        artifact_type: ArtifactType::Logs,
        content_type: "text/plain".to_string(),
        size_bytes: 1_048_576, // 1 MB
        checksum: "sha256:logs".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: None,
        tags: HashMap::new(),
        execution_id: None,
        storage_info: StorageInfo {
            node_id: "archive-node".to_string(),
            path: "/archive/logs/2025".to_string(),
            tier: StorageTier::Archive,
            replicated: true,
            compression: CompressionType::Zstd,
            encryption: EncryptionType::Default,
        },
        version: None,
    };

    assert!(matches!(metadata.storage_info.tier, StorageTier::Archive));
    assert!(matches!(
        metadata.storage_info.compression,
        CompressionType::Zstd
    ));
}

#[test]
fn test_scenario_wasm_module_deployment() {
    let metadata = ArtifactMetadata {
        id: "service.wasm".to_string(),
        artifact_type: ArtifactType::WasmModule,
        content_type: "application/wasm".to_string(),
        size_bytes: 204_800, // 200 KB
        checksum: "sha256:wasm".to_string(),
        created_at: std::time::SystemTime::now(),
        last_accessed: Some(std::time::SystemTime::now()),
        tags: {
            let mut tags = HashMap::new();
            tags.insert("service".to_string(), "api".to_string());
            tags
        },
        execution_id: Some(Uuid::new_v4()),
        storage_info: StorageInfo {
            node_id: "edge-node".to_string(),
            path: "/wasm/services".to_string(),
            tier: StorageTier::Hot,
            replicated: true,
            compression: CompressionType::None,
            encryption: EncryptionType::None,
        },
        version: None,
    };

    assert!(matches!(metadata.artifact_type, ArtifactType::WasmModule));
    assert_eq!(metadata.content_type, "application/wasm");
}
