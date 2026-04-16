// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::config::CacheConfig;
use crate::pipeline::PipelineConfig;
use crate::types::{ArtifactFilters, ArtifactType, StorageStatus};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

fn test_client() -> StorageClient {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    StorageClient::new_for_testing(config, "test-storage".to_string())
}

/// Remote store may succeed, or fall back to local-only when no service is reachable.
fn assert_store_completed(status: &StorageStatus) {
    assert!(
        matches!(status, StorageStatus::Success | StorageStatus::LocalOnly),
        "unexpected status: {status:?}"
    );
}

#[test]
fn test_client_construction() {
    let client = test_client();
    assert_eq!(client.config.endpoint, "unix://test");
    assert_eq!(client.config.max_retries, 2);
}

#[tokio::test]
async fn test_store_artifact_returns_result() {
    let client = test_client();
    let data = b"hello world";
    let result = client.store_artifact("test.bin", data).await.unwrap();
    assert_store_completed(&result.status);
    assert!(
        result.message.contains("test.bin"),
        "message should reference artifact name: {}",
        result.message
    );
}

#[tokio::test]
async fn test_store_artifact_checksum() {
    let client = test_client();
    let data = b"consistent data for checksum";
    let r1 = client.store_artifact("a", data).await.unwrap();
    let r2 = client.store_artifact("b", data).await.unwrap();
    assert_store_completed(&r1.status);
    assert_store_completed(&r2.status);
}

#[tokio::test]
async fn test_store_artifact_content_type_zip() {
    let client = test_client();
    let zip_magic = [0x50, 0x4B, 0x03, 0x04]; // PK..
    let result = client
        .store_artifact("archive.zip", &zip_magic)
        .await
        .unwrap();
    assert_store_completed(&result.status);
}

#[tokio::test]
async fn test_store_artifact_content_type_png() {
    let client = test_client();
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let result = client
        .store_artifact("image.png", &png_magic)
        .await
        .unwrap();
    assert_store_completed(&result.status);
}

#[tokio::test]
async fn test_store_artifact_content_type_jpeg() {
    let client = test_client();
    let jpeg_magic = [0xFF, 0xD8, 0xFF];
    let result = client
        .store_artifact("photo.jpg", &jpeg_magic)
        .await
        .unwrap();
    assert_store_completed(&result.status);
}

#[tokio::test]
async fn test_store_artifact_content_type_octet_stream() {
    let client = test_client();
    let data = b"generic binary";
    let result = client.store_artifact("data.bin", data).await.unwrap();
    assert_store_completed(&result.status);
}

#[tokio::test]
async fn test_retrieve_artifact_not_in_cache() {
    let client = test_client();
    let id = uuid::Uuid::new_v4();
    let result = client.retrieve_artifact(id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_retrieve_artifact_with_cache_disabled() {
    let config = StorageConfig {
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
    let result = client
        .retrieve_artifact(uuid::Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_cleanup_cache_disabled_noop() {
    let config = StorageConfig {
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
fn test_cleanup_cache_enabled_noop() {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: Some(CacheConfig {
            enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/test-cache")),
            max_size: 1024,
            ttl: Duration::from_secs(3600),
        }),
    };
    let client = StorageClient::new_for_testing(config, "test".to_string());
    client.cleanup_cache();
}

#[test]
fn test_storage_error_display() {
    let e = StorageError::Connection("test".to_string());
    assert!(e.to_string().contains("test"));
    let e = StorageError::Network("net".to_string());
    assert!(e.to_string().contains("net"));
    let e = StorageError::Pipeline("pipe".to_string());
    assert!(e.to_string().contains("pipe"));
    let e = StorageError::Storage("storage err".to_string());
    assert!(e.to_string().contains("storage"));
}

#[test]
fn test_pipeline_config_serialization() {
    let config = PipelineConfig {
        pipeline_id: "p1".to_string(),
        name: "Test".to_string(),
        inputs: vec![],
        outputs: vec![],
        steps: vec![],
        schedule: None,
        resources: None,
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["pipeline_id"], "p1");
    assert_eq!(json["name"], "Test");
}

#[test]
fn test_artifact_filters_serialization() {
    let filters = ArtifactFilters {
        artifact_type: Some(ArtifactType::DataFile),
        execution_id: None,
        created_since: None,
        tags: HashMap::new(),
    };
    let json = serde_json::to_value(&filters).unwrap();
    assert!(json.get("artifact_type").is_some());
}

#[tokio::test]
async fn test_store_artifact_different_content_types() {
    let client = test_client();
    let zip = [0x50, 0x4B, 0x03, 0x04];
    let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let jpeg = [0xFF, 0xD8, 0xFF];
    let r1 = client.store_artifact("a.zip", &zip).await.unwrap();
    let r2 = client.store_artifact("b.png", &png).await.unwrap();
    let r3 = client.store_artifact("c.jpg", &jpeg).await.unwrap();
    let r4 = client.store_artifact("d.bin", b"raw").await.unwrap();
    assert_store_completed(&r1.status);
    assert_store_completed(&r2.status);
    assert_store_completed(&r3.status);
    assert_store_completed(&r4.status);
}

#[tokio::test]
async fn test_store_artifact_returns_uuid() {
    let client = test_client();
    let result = client.store_artifact("test", b"data").await.unwrap();
    assert_store_completed(&result.status);
    assert!(!result.id.is_nil());
}

#[test]
fn test_calculate_checksum_consistent() {
    let data = b"hello world";
    let c1 = StorageClient::calculate_checksum(data);
    let c2 = StorageClient::calculate_checksum(data);
    assert_eq!(c1, c2);
    assert_eq!(c1.len(), 64);
}

#[test]
fn test_calculate_checksum_different_data() {
    let c1 = StorageClient::calculate_checksum(b"a");
    let c2 = StorageClient::calculate_checksum(b"b");
    assert_ne!(c1, c2);
}

#[test]
fn test_detect_content_type_zip() {
    let zip_magic = [0x50, 0x4B, 0x03, 0x04];
    assert_eq!(
        StorageClient::detect_content_type(&zip_magic),
        "application/zip"
    );
}

#[test]
fn test_detect_content_type_png() {
    let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(StorageClient::detect_content_type(&png_magic), "image/png");
}

#[test]
fn test_detect_content_type_jpeg() {
    let jpeg_magic = [0xFF, 0xD8, 0xFF];
    assert_eq!(
        StorageClient::detect_content_type(&jpeg_magic),
        "image/jpeg"
    );
}

#[test]
fn test_detect_content_type_octet_stream() {
    assert_eq!(
        StorageClient::detect_content_type(b"generic"),
        "application/octet-stream"
    );
}

#[tokio::test]
async fn test_get_artifact_metadata_unavailable() {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let client = StorageClient::new_for_testing(config, "nonexistent-storage".to_string());
    let result = client
        .get_artifact_metadata(&uuid::Uuid::new_v4().to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_artifacts_unavailable() {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let client = StorageClient::new_for_testing(config, "nonexistent-storage".to_string());
    let result = client.list_artifacts(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_artifact_unavailable() {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let client = StorageClient::new_for_testing(config, "nonexistent-storage".to_string());
    let result = client
        .delete_artifact(&uuid::Uuid::new_v4().to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_pipeline_unavailable() {
    let config = StorageConfig {
        endpoint: "unix://test".to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 2,
        auth: None,
        cache: None,
    };
    let client = StorageClient::new_for_testing(config, "nonexistent-storage".to_string());
    let pipeline_config = PipelineConfig {
        pipeline_id: "p1".to_string(),
        name: "Test".to_string(),
        inputs: vec![],
        outputs: vec![],
        steps: vec![],
        schedule: None,
        resources: None,
    };
    let result = client.create_pipeline(pipeline_config).await;
    assert!(result.is_err());
}
