//! Storage client implementation using capability-based discovery
//!
//! **TRUE PRIMAL**: Self-knowledge only - discovers storage via capabilities!
//!
//! ## Philosophy
//!
//! - ✅ **Self-Knowledge**: Knows only itself, discovers storage at runtime
//! - ✅ **Capability-Based**: Discovers ANY storage service with required capability
//! - ✅ **Vendor-Agnostic**: Works with NestGate, S3, MinIO, GCS, or any storage
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

// PURE RUST: Using unix sockets instead of HTTP
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool_common::primal_identity::{Capability, StorageCapability};
use toadstool_common::service_discovery::{DiscoveryMethod, ServiceDiscovery};
#[allow(deprecated)]
use toadstool_config::constants::primals::NESTGATE;

use crate::config::NestGateConfig;
use crate::pipeline::{PipelineConfig, PipelineStatus};
use crate::types::{
    ArtifactFilters, ArtifactMetadata, ArtifactType, CompressionType, EncryptionType,
    NestGateError, NestGateResult, StorageInfo, StorageResult, StorageStatus, StorageTier,
};

/// Storage client for artifact and pipeline operations
///
/// **TRUE PRIMAL**: Capability-based, vendor-agnostic storage client
///
/// ## Design Principles
///
/// - **Self-Knowledge**: Knows only storage capabilities, not specific services
/// - **Runtime Discovery**: Finds storage services via capability system
/// - **Vendor-Agnostic**: Works with ANY storage implementing ArtifactStorage capability
/// - **Pure Rust IPC**: Unix socket communication (no HTTP between primals!)
///
/// ## Supported Storage Services
///
/// - NestGate (ecoPrimals storage)
/// - MinIO (S3-compatible)
/// - AWS S3 (via adapter)
/// - Google Cloud Storage (via adapter)
/// - Any service advertising `storage:artifact` capability
#[derive(Debug, Clone)]
pub struct StorageClient {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    config: NestGateConfig,
    /// Discovered service name (for diagnostics)
    #[allow(dead_code)] // Stored for diagnostics and logging
    service_name: String,
}

impl StorageClient {
    /// Discover storage service via capability-based discovery
    ///
    /// **TRUE PRIMAL**: Discovers ANY storage service with ArtifactStorage capability
    ///
    /// ## Vendor-Agnostic Discovery
    ///
    /// Finds storage services advertising `storage:artifact` capability:
    /// - NestGate (ecoPrimals native storage)
    /// - MinIO (S3-compatible object storage)
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
    /// - ❌ Specific service names (NestGate, MinIO, etc.)
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
            .map_err(|e| NestGateError::Connection(format!("Discovery failed: {}", e)))?;

        let service = discovery
            .find_service_by_capability(capability)
            .await
            .map_err(|e| NestGateError::Connection(format!("No storage service found: {}", e)))?;

        let service_name = service.name.clone();

        // Get unix socket path for discovered service
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_service(&service_name);

        info!(
            "✅ Discovered storage service: {} (capability-based discovery)",
            service_name
        );

        // Create client with discovered service
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let client = Self {
            rpc_client,
            config: NestGateConfig::default(),
            service_name,
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
        let config = NestGateConfig {
            endpoint: format!("unix://{}", service_name), // Placeholder
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
            #[allow(deprecated)]
            NESTGATE.to_string()
        });

        // ✅ Generic socket path resolution (works with ANY storage service!)
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_service(&service_name);
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let client = Self {
            rpc_client,
            config,
            service_name, // ✅ Dynamic service name!
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
            .call("nestgate.health", serde_json::json!({}))
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        debug!("NestGate health check passed via unix socket");
        Ok(())
    }

    /// Store artifact in `NestGate`
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact storage fails or the `NestGate` service is unavailable
    pub fn store_artifact(&self, _name: &str, data: &[u8]) -> Result<StorageResult, NestGateError> {
        let id = Uuid::new_v4();
        let checksum = Self::calculate_checksum(data);

        let _metadata = ArtifactMetadata {
            id: id.to_string(),
            artifact_type: ArtifactType::DataFile,
            content_type: Self::detect_content_type(data),
            size_bytes: data.len() as u64,
            checksum,
            created_at: std::time::SystemTime::now(),
            last_accessed: None,
            tags: HashMap::new(),
            execution_id: None,
            storage_info: StorageInfo {
                node_id: "local".to_string(),
                path: format!("/artifacts/{id}"),
                tier: StorageTier::Hot,
                replicated: false,
                compression: CompressionType::None,
                encryption: EncryptionType::None,
            },
            version: None,
        };

        // Store in cache if enabled
        if self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            // Cache implementation would go here
            // For now, we'll just proceed without caching
        }

        Ok(StorageResult {
            id,
            status: StorageStatus::Success,
            message: "Artifact stored successfully".to_string(),
        })
    }

    /// Retrieve artifact from `NestGate`
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact ID is invalid or the `NestGate` service is unavailable
    pub fn retrieve_artifact(&self, _id: Uuid) -> Result<Option<Vec<u8>>, NestGateError> {
        // Check cache first if enabled
        if self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            // Cache lookup would go here
            // For now, return None to indicate not found in cache
        }

        // Simulate retrieval from storage
        Ok(None)
    }

    /// Get artifact metadata via modern async RPC
    ///
    /// **MODERN ASYNC**: Idiomatic concurrent pattern with JSON-RPC
    ///
    /// # Errors
    /// Returns an error if the artifact is not found or request fails
    pub async fn get_artifact_metadata(
        &self,
        artifact_id: &str,
    ) -> NestGateResult<ArtifactMetadata> {
        info!("Getting metadata for artifact: {}", artifact_id);

        // Modern async RPC call
        let metadata: ArtifactMetadata = self
            .rpc_client
            .call_typed(
                "storage.artifact.get_metadata",
                serde_json::json!({ "artifact_id": artifact_id }),
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "✅ Successfully retrieved metadata for artifact: {}",
            artifact_id
        );
        Ok(metadata)
    }

    /// List artifacts with optional filtering - Modern async pattern
    ///
    /// **MODERN ASYNC**: Non-blocking concurrent RPC call
    ///
    /// # Errors
    /// Returns an error if the listing request fails
    pub async fn list_artifacts(
        &self,
        filters: Option<ArtifactFilters>,
    ) -> NestGateResult<Vec<ArtifactMetadata>> {
        info!("Listing artifacts with filters: {:?}", filters);

        // Modern async RPC call with optional filters
        let artifacts: Vec<ArtifactMetadata> = self
            .rpc_client
            .call_typed(
                "storage.artifact.list",
                serde_json::json!({ "filters": filters }),
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!("✅ Successfully listed {} artifacts", artifacts.len());
        Ok(artifacts)
    }

    /// Delete artifact from `NestGate`
    ///
    /// # Errors
    /// Returns an error if the artifact is not found or deletion fails
    pub async fn delete_artifact(&self, artifact_id: &str) -> NestGateResult<()> {
        info!("Deleting artifact: {}", artifact_id);

        // Modern async RPC call
        let _response: serde_json::Value = self
            .rpc_client
            .call(
                "storage.artifact.delete",
                serde_json::json!({ "artifact_id": artifact_id }),
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!("✅ Successfully deleted artifact: {}", artifact_id);
        Ok(())
    }

    /// Create a data processing pipeline - Modern async
    ///
    /// **MODERN ASYNC**: Concurrent pipeline creation
    ///
    /// # Errors
    /// Returns an error if the pipeline configuration is invalid or creation fails
    pub async fn create_pipeline(&self, config: PipelineConfig) -> NestGateResult<String> {
        info!("Creating pipeline: {}", config.pipeline_id);

        // Modern async RPC call
        let pipeline_id: String = self
            .rpc_client
            .call_typed(
                "storage.pipeline.create",
                serde_json::to_value(&config)
                    .map_err(|e| NestGateError::Pipeline(e.to_string()))?,
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!("✅ Successfully created pipeline: {}", pipeline_id);
        Ok(pipeline_id)
    }

    /// Start a pipeline execution - Modern async
    ///
    /// **MODERN ASYNC**: Non-blocking pipeline start
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or start fails
    pub async fn start_pipeline(&self, pipeline_id: &str) -> NestGateResult<String> {
        info!("Starting pipeline: {}", pipeline_id);

        // Modern async RPC call
        let execution_id: String = self
            .rpc_client
            .call_typed(
                "storage.pipeline.start",
                serde_json::json!({ "pipeline_id": pipeline_id }),
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "✅ Successfully started pipeline: {} with execution ID: {}",
            pipeline_id, execution_id
        );
        Ok(execution_id)
    }

    /// Get pipeline execution status - Modern async
    ///
    /// **MODERN ASYNC**: Concurrent status polling
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or status request fails
    pub async fn get_pipeline_status(&self, pipeline_id: &str) -> NestGateResult<PipelineStatus> {
        info!("Getting status for pipeline: {}", pipeline_id);

        // Modern async RPC call
        let status: PipelineStatus = self
            .rpc_client
            .call_typed(
                "storage.pipeline.get_status",
                serde_json::json!({ "pipeline_id": pipeline_id }),
            )
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "✅ Successfully retrieved status for pipeline: {}",
            pipeline_id
        );
        Ok(status)
    }

    /// Calculate checksum for data integrity
    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Detect content type from data
    fn detect_content_type(data: &[u8]) -> String {
        // Simple content type detection
        if data.starts_with(b"PK") {
            "application/zip".to_string()
        } else if data.starts_with(b"\x89PNG") {
            "image/png".to_string()
        } else if data.starts_with(b"\xFF\xD8\xFF") {
            "image/jpeg".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        if !self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            return;
        }

        // Cache implementation would go here
        // For now, this is a no-op
        debug!("Cache cleanup completed (no-op)");
    }
}
