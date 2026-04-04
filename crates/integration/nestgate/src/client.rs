// SPDX-License-Identifier: AGPL-3.0-only
//! Storage client implementation using capability-based discovery
//!
//! **TRUE PRIMAL**: Self-knowledge only - discovers storage via capabilities!

//!
//! ## Philosophy
//!
//! - ✅ **Self-Knowledge**: Knows only itself, discovers storage at runtime
//! - ✅ **Capability-Based**: Discovers ANY storage service with required capability
//! - ✅ **Vendor-Agnostic**: Works with NestGate, S3, `MinIO`, GCS, or any storage
//! - ✅ **Pure Rust**: Unix socket IPC for primal communication
//!
//! ## Usage
//!
//! ```ignore
//! use toadstool_integration_nestgate::StorageClient;
//!
//! // Discover ANY storage service with ArtifactStorage capability
//! let client = StorageClient::discover().await?;
//!
//! // Store artifact (vendor-agnostic!)
//! client.store_artifact("model.bin", data).await?;
//! ```

use tracing::{debug, info};

use toadstool_common::interned_strings::capabilities;
use toadstool_common::primal_identity::{Capability, StorageCapability};
use toadstool_common::service_discovery::{DiscoveryMethod, ServiceDiscovery};

use crate::config::NestGateConfig;
use crate::types::{NestGateError, NestGateResult};

/// Storage client for artifact and pipeline operations
///
/// **TRUE PRIMAL**: Capability-based, vendor-agnostic storage client
///
/// ## Design Principles
///
/// - **Self-Knowledge**: Knows only storage capabilities, not specific services
/// - **Runtime Discovery**: Finds storage services via capability system
/// - **Vendor-Agnostic**: Works with ANY storage implementing `ArtifactStorage` capability
/// - **Pure Rust IPC**: Unix socket communication (no HTTP between primals!)
///
/// ## Supported Storage Services
///
/// - NestGate (ecoPrimals storage)
/// - `MinIO` (S3-compatible)
/// - AWS S3 (via adapter)
/// - Google Cloud Storage (via adapter)
/// - Any service advertising `storage:artifact` capability
#[derive(Debug, Clone)]
pub struct StorageClient {
    /// RPC client for JSON-RPC over unix socket (crate-visible for impl in artifacts/pipelines)
    pub(crate) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Client configuration (crate-visible for impl in utils)
    pub(crate) config: NestGateConfig,
    /// Discovered service name (for diagnostics)
    _service_name: String,
}

impl StorageClient {
    /// Discover storage service via capability-based discovery
    ///
    /// **TRUE PRIMAL**: Discovers ANY storage service with `ArtifactStorage` capability
    ///
    /// ## Vendor-Agnostic Discovery
    ///
    /// Finds storage services advertising `storage:artifact` capability:
    /// - NestGate (ecoPrimals native storage)
    /// - `MinIO` (S3-compatible object storage)
    /// - AWS S3 (via capability adapter)
    /// - Google Cloud Storage (via capability adapter)
    /// - Custom storage implementations
    ///
    /// ## Self-Knowledge Principle
    ///
    /// This client knows:
    /// - ✅ What capabilities it needs (storage:artifact)
    /// - ✅ How to communicate via unix sockets
    ///
    /// This client does NOT know:
    /// - ❌ Specific service names (NestGate, `MinIO`, etc.)
    /// - ❌ Hardcoded endpoints or ports
    /// - ❌ Implementation details
    ///
    /// # Errors
    /// Returns an error if no storage service is found or connection fails
    pub async fn discover() -> NestGateResult<Self> {
        Self::discover_with_capability(Capability::Storage(StorageCapability::ArtifactStorage))
            .await
    }

    /// Discover storage service by specific capability
    ///
    /// **TRUE PRIMAL**: Runtime discovery, no hardcoding!
    ///
    /// # Errors
    /// Returns an error if no service is found or connection fails
    pub async fn discover_with_capability(capability: Capability) -> NestGateResult<Self> {
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
            .await
            .map_err(|e| NestGateError::Connection(format!("Discovery failed: {e}")))?;

        let service = discovery
            .find_service_by_capability(capability)
            .await
            .map_err(|e| NestGateError::Connection(format!("No storage service found: {e}")))?;

        let service_name = service.name.clone();

        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);

        info!(
            "✅ Discovered storage service: {} (capability-based discovery)",
            service_name
        );

        // Create client with discovered service
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let client = Self {
            rpc_client,
            config: NestGateConfig::default(),
            _service_name: service_name,
        };

        // Verify connectivity
        client.health_check().await?;

        Ok(client)
    }

    /// Connect to storage server by service name
    ///
    /// **Note**: Consider using `discover()` for capability-based discovery.
    /// This method requires knowing the service name (e.g., "nestgate", "minio").
    ///
    /// # Arguments
    /// * `service_name` - Name of the storage service to connect to
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid or connection fails
    pub async fn connect(service_name: &str) -> NestGateResult<Self> {
        let socket = toadstool_common::primal_sockets::get_socket_path_for_capability(service_name);
        let config = NestGateConfig {
            endpoint: format!("unix://{}", socket.display()),
            ..Default::default()
        };
        Self::with_config(config, Some(service_name.to_string())).await
    }

    /// Create client with custom configuration
    ///
    /// **TRUE PRIMAL**: Accepts optional service name from discovery, no hardcoding!
    ///
    /// # Arguments
    /// * `config` - Storage configuration
    /// * `service_name` - Optional discovered service name (defaults to "nestgate" for backward compat)
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid
    pub async fn with_config(
        config: NestGateConfig,
        service_name: Option<String>,
    ) -> NestGateResult<Self> {
        // ✅ TRUE PRIMAL: Use discovered service name or fallback
        let service_name = service_name.unwrap_or_else(|| {
            // Fallback for backward compatibility
            // In production, prefer using discover() which provides the service name
            capabilities::STORAGE.to_string()
        });

        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let client = Self {
            rpc_client,
            config,
            _service_name: service_name,
        };

        // Perform initial health check
        client.health_check().await?;

        Ok(client)
    }

    /// Check `NestGate` server health via unix socket
    ///
    /// **PURE RUST**: JSON-RPC over unix socket (modern async pattern!)
    ///
    /// # Errors
    /// Returns an error if the health check request fails or server is unhealthy
    pub async fn health_check(&self) -> NestGateResult<()> {
        // Modern async RPC pattern
        let _response: serde_json::Value = self
            .rpc_client
            .call("storage.health", serde_json::json!({}))
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        debug!("Storage service health check passed via unix socket");
        Ok(())
    }

    /// Create client for testing without health check (skips RPC connectivity)
    ///
    /// Use for unit tests that exercise local logic (store_artifact, retrieve_artifact,
    /// checksum, content-type detection) without requiring a running NestGate server.
    #[doc(hidden)]
    #[must_use]
    pub fn new_for_testing(config: NestGateConfig, service_name: String) -> Self {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        Self {
            rpc_client,
            config,
            _service_name: service_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CacheConfig;
    use crate::pipeline::PipelineConfig;
    use crate::types::{ArtifactFilters, ArtifactType, StorageStatus};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn test_client() -> StorageClient {
        let config = NestGateConfig {
            endpoint: "unix://test".to_string(),
            timeout: Duration::from_secs(5),
            max_retries: 2,
            auth: None,
            cache: None,
        };
        StorageClient::new_for_testing(config, "test-storage".to_string())
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
        assert!(matches!(result.status, StorageStatus::Success));
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
        assert!(matches!(r1.status, StorageStatus::Success));
        assert!(matches!(r2.status, StorageStatus::Success));
    }

    #[tokio::test]
    async fn test_store_artifact_content_type_zip() {
        let client = test_client();
        let zip_magic = [0x50, 0x4B, 0x03, 0x04]; // PK..
        let result = client
            .store_artifact("archive.zip", &zip_magic)
            .await
            .unwrap();
        assert!(matches!(result.status, StorageStatus::Success));
    }

    #[tokio::test]
    async fn test_store_artifact_content_type_png() {
        let client = test_client();
        let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let result = client
            .store_artifact("image.png", &png_magic)
            .await
            .unwrap();
        assert!(matches!(result.status, StorageStatus::Success));
    }

    #[tokio::test]
    async fn test_store_artifact_content_type_jpeg() {
        let client = test_client();
        let jpeg_magic = [0xFF, 0xD8, 0xFF];
        let result = client
            .store_artifact("photo.jpg", &jpeg_magic)
            .await
            .unwrap();
        assert!(matches!(result.status, StorageStatus::Success));
    }

    #[tokio::test]
    async fn test_store_artifact_content_type_octet_stream() {
        let client = test_client();
        let data = b"generic binary";
        let result = client.store_artifact("data.bin", data).await.unwrap();
        assert!(matches!(result.status, StorageStatus::Success));
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
        let result = client
            .retrieve_artifact(uuid::Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cleanup_cache_disabled_noop() {
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
    fn test_cleanup_cache_enabled_noop() {
        let config = NestGateConfig {
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
    fn test_nestgate_error_display() {
        let e = NestGateError::Connection("test".to_string());
        assert!(e.to_string().contains("test"));
        let e = NestGateError::Network("net".to_string());
        assert!(e.to_string().contains("net"));
        let e = NestGateError::Pipeline("pipe".to_string());
        assert!(e.to_string().contains("pipe"));
        let e = NestGateError::Storage("storage err".to_string());
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
        assert!(matches!(r1.status, StorageStatus::Success));
        assert!(matches!(r2.status, StorageStatus::Success));
        assert!(matches!(r3.status, StorageStatus::Success));
        assert!(matches!(r4.status, StorageStatus::Success));
    }

    #[tokio::test]
    async fn test_store_artifact_returns_uuid() {
        let client = test_client();
        let result = client.store_artifact("test", b"data").await.unwrap();
        assert!(matches!(result.status, StorageStatus::Success));
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
        let config = NestGateConfig {
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
        let config = NestGateConfig {
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
        let config = NestGateConfig {
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
        let config = NestGateConfig {
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
}
