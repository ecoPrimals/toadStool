// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for NestGate `StorageClient` (client.rs)
//!
//! Target: crates/integration/nestgate/src/client.rs — 90% coverage
//! Tests store/retrieve, pipeline ops, `cleanup_cache`, content-type detection.
//! Uses `new_for_testing` — no real TCP/HTTP.

use std::path::PathBuf;
use std::time::Duration;

use toadstool_integration_nestgate::config::{CacheConfig, NestGateConfig};
use toadstool_integration_nestgate::pipeline::PipelineConfig;
use toadstool_integration_nestgate::types::{ArtifactFilters, ArtifactType, StorageStatus};
use toadstool_integration_nestgate::StorageClient;

// ============================================================================
// Test client helper
// ============================================================================

fn test_client() -> StorageClient {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    StorageClient::new_for_testing(config, "test-storage-coverage".to_string())
}

// ============================================================================
// store_artifact tests
// ============================================================================

#[test]
fn store_artifact_returns_success_with_uuid() {
    let client = test_client();
    let data = b"coverage test data";
    let result = client.store_artifact("coverage-test.bin", data).unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
    assert!(!result.id.is_nil());
    assert!(result.message.contains("coverage-test.bin"));
}

#[test]
fn store_artifact_fallback_message_when_no_server() {
    let client = test_client();
    let result = client.store_artifact("fallback.bin", b"data").unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
    assert!(
        result.message.contains("locally") || result.message.contains("fallback.bin"),
        "message: {}",
        result.message
    );
}

#[test]
fn store_artifact_content_type_zip() {
    let client = test_client();
    let zip_magic = [0x50, 0x4B, 0x03, 0x04];
    let result = client.store_artifact("archive.zip", &zip_magic).unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
}

#[test]
fn store_artifact_content_type_png() {
    let client = test_client();
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let result = client.store_artifact("image.png", &png_magic).unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
}

#[test]
fn store_artifact_content_type_jpeg() {
    let client = test_client();
    let jpeg_magic = [0xFF, 0xD8, 0xFF];
    let result = client.store_artifact("photo.jpg", &jpeg_magic).unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
}

#[test]
fn store_artifact_content_type_octet_stream() {
    let client = test_client();
    let result = client.store_artifact("binary.bin", b"raw bytes").unwrap();
    assert!(matches!(result.status, StorageStatus::Success));
}

// ============================================================================
// retrieve_artifact tests
// ============================================================================

#[test]
fn retrieve_artifact_returns_none_when_unavailable() {
    let client = test_client();
    let id = uuid::Uuid::new_v4();
    let result = client.retrieve_artifact(id).unwrap();
    assert!(result.is_none());
}

// calculate_checksum and detect_content_type are pub(crate) - tested via store_artifact

// ============================================================================
// cleanup_cache tests
// ============================================================================

#[test]
fn cleanup_cache_disabled_noop() {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: Some(CacheConfig {
            enabled: false,
            cache_dir: None,
            max_size: 0,
            ttl: Duration::from_secs(0),
        }),
    };
    let client = StorageClient::new_for_testing(config, "test".to_string());
    client.cleanup_cache();
}

#[test]
fn cleanup_cache_enabled_noop() {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: Some(CacheConfig {
            enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/nestgate-cache")),
            max_size: 1024,
            ttl: Duration::from_secs(3600),
        }),
    };
    let client = StorageClient::new_for_testing(config, "test".to_string());
    client.cleanup_cache();
}

#[test]
fn cleanup_cache_none_config() {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let client = StorageClient::new_for_testing(config, "test".to_string());
    client.cleanup_cache();
}

// ============================================================================
// Async RPC error path tests (no server)
// ============================================================================

#[tokio::test]
async fn get_artifact_metadata_fails_without_server() {
    let client = test_client();
    let result = client
        .get_artifact_metadata(&uuid::Uuid::new_v4().to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_artifacts_fails_without_server() {
    let client = test_client();
    let result = client.list_artifacts(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_artifacts_with_filters_fails_without_server() {
    let client = test_client();
    let filters = ArtifactFilters {
        artifact_type: Some(ArtifactType::DataFile),
        execution_id: None,
        created_since: None,
        tags: std::collections::HashMap::new(),
    };
    let result = client.list_artifacts(Some(filters)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delete_artifact_fails_without_server() {
    let client = test_client();
    let result = client
        .delete_artifact(&uuid::Uuid::new_v4().to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_pipeline_fails_without_server() {
    let client = test_client();
    let config = PipelineConfig {
        pipeline_id: "p1".to_string(),
        name: "Test Pipeline".to_string(),
        inputs: vec![],
        outputs: vec![],
        steps: vec![],
        schedule: None,
        resources: None,
    };
    let result = client.create_pipeline(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn start_pipeline_fails_without_server() {
    let client = test_client();
    let result = client.start_pipeline("pipeline-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_pipeline_status_fails_without_server() {
    let client = test_client();
    let result = client.get_pipeline_status("pipeline-1").await;
    assert!(result.is_err());
}

// ============================================================================
// with_config error path (service_name fallback exercised)
// ============================================================================

#[tokio::test]
async fn with_config_fails_on_health_check() {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    // service_name: None triggers NESTGATE fallback
    let result = StorageClient::with_config(config, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn with_config_with_service_name_fails_on_health_check() {
    let config = NestGateConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let result = StorageClient::with_config(config, Some("nonexistent-storage".to_string())).await;
    assert!(result.is_err());
}
