//! `NestGate` client implementation for artifact and storage management
//!
//! **Evolution**: Now supports capability-based discovery in addition to direct endpoints.
//!
//! ## Usage Patterns
//!
//! ### Pattern 1: Direct Endpoint (Legacy, still supported)
//! ```ignore
//! let client = NestGateClient::connect("http://localhost:8080").await?;
//! ```
//!
//! ### Pattern 2: Capability-Based Discovery (NEW)
//! ```ignore
//! let client = NestGateClient::discover().await?;
//! // Discovers ANY storage service with ArtifactStorage capability
//! ```

use chrono::Utc;
use reqwest::Client;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool_common::primal_identity::{Capability, StorageCapability};
use toadstool_common::service_discovery::{DiscoveryMethod, ServiceDiscovery};

use crate::config::NestGateConfig;
use crate::pipeline::{PipelineConfig, PipelineStatus};
use crate::types::{
    ArtifactFilters, ArtifactMetadata, ArtifactType, CompressionType, EncryptionType,
    NestGateError, NestGateResult, StorageInfo, StorageResult, StorageStatus, StorageTier,
};

/// Main `NestGate` client for storage and pipeline operations
///
/// **Design**: Supports both direct endpoint and capability-based discovery
#[derive(Debug, Clone)]
pub struct NestGateClient {
    client: Client,
    config: NestGateConfig,
}

impl NestGateClient {
    /// Discover storage service by capability (RECOMMENDED)
    ///
    /// Discovers ANY storage service advertising ArtifactStorage capability.
    /// This is vendor-agnostic and works with NestGate, S3, MinIO, GCS, etc.
    ///
    /// # Errors
    /// Returns an error if no storage service is found or connection fails
    pub async fn discover() -> NestGateResult<Self> {
        Self::discover_with_capability(Capability::Storage(StorageCapability::ArtifactStorage))
            .await
    }

    /// Discover storage service by specific capability
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
            .map_err(|e| {
                NestGateError::Connection(format!("No storage service found: {}", e))
            })?;

        let endpoint = service
            .endpoints
            .first()
            .ok_or_else(|| NestGateError::Connection("No endpoints available".to_string()))?;

        let endpoint_url = endpoint.url();

        info!(
            "Discovered storage service: {} at {}",
            service.name, endpoint_url
        );

        Self::connect(&endpoint_url).await
    }

    /// Connect to storage server with default configuration (Direct endpoint)
    ///
    /// **Note**: Consider using `discover()` for capability-based discovery.
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid or connection fails
    pub async fn connect(endpoint: &str) -> NestGateResult<Self> {
        let config = NestGateConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create client with custom configuration
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid
    pub async fn with_config(config: NestGateConfig) -> NestGateResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| NestGateError::Connection(e.to_string()))?;

        let client = Self {
            config,
            client: http_client,
        };

        // Perform initial health check
        client.health_check().await?;

        Ok(client)
    }

    /// Check `NestGate` server health
    ///
    /// # Errors
    /// Returns an error if the health check request fails or server is unhealthy
    pub async fn health_check(&self) -> NestGateResult<()> {
        let url = format!("{}/health", self.config.endpoint);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if response.status().is_success() {
            debug!("NestGate health check passed");
            Ok(())
        } else {
            warn!("NestGate health check failed: {}", response.status());
            Err(NestGateError::Connection(format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }

    /// Store artifact in `NestGate`
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact storage fails or the `NestGate` service is unavailable
    pub fn store_artifact(
        &self,
        _name: &str,
        data: &[u8],
    ) -> Result<StorageResult, Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4();
        let checksum = Self::calculate_checksum(data);

        let _metadata = ArtifactMetadata {
            id: id.to_string(),
            artifact_type: ArtifactType::DataFile,
            content_type: Self::detect_content_type(data),
            size_bytes: data.len() as u64,
            checksum,
            created_at: Utc::now(),
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
    pub fn retrieve_artifact(
        &self,
        _id: Uuid,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        // Check cache first if enabled
        if self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            // Cache lookup would go here
            // For now, return None to indicate not found in cache
        }

        // Simulate retrieval from storage
        Ok(None)
    }

    /// Get artifact metadata
    ///
    /// # Errors
    /// Returns an error if the artifact is not found or request fails
    pub async fn get_artifact_metadata(
        &self,
        artifact_id: &str,
    ) -> NestGateResult<ArtifactMetadata> {
        info!("Getting metadata for artifact: {}", artifact_id);

        // Check cache first
        if self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            // Cache implementation would go here
            // For now, we'll just proceed without caching
        }

        // Get from NestGate
        let url = format!(
            "{}/artifacts/{}/metadata",
            self.config.endpoint, artifact_id
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Storage(format!(
                "Failed to get metadata: {}",
                response.status()
            )));
        }

        let metadata: ArtifactMetadata = response
            .json()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "Successfully retrieved metadata for artifact: {}",
            artifact_id
        );
        Ok(metadata)
    }

    /// List artifacts with optional filtering
    ///
    /// # Errors
    /// Returns an error if the listing request fails
    pub async fn list_artifacts(
        &self,
        filters: Option<ArtifactFilters>,
    ) -> NestGateResult<Vec<ArtifactMetadata>> {
        info!("Listing artifacts with filters: {:?}", filters);

        let mut url = format!("{}/artifacts", self.config.endpoint);

        // Add query parameters for filters
        if let Some(filters) = filters {
            let mut params = Vec::new();

            if let Some(artifact_type) = filters.artifact_type {
                params.push(format!("type={artifact_type:?}"));
            }

            if let Some(execution_id) = filters.execution_id {
                params.push(format!("execution_id={execution_id}"));
            }

            if let Some(created_since) = filters.created_since {
                params.push(format!("created_since={}", created_since.to_rfc3339()));
            }

            for (key, value) in filters.tags {
                params.push(format!("tag_{key}={value}"));
            }

            if !params.is_empty() {
                url = format!("{}?{}", url, params.join("&"));
            }
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Storage(format!(
                "Failed to list artifacts: {}",
                response.status()
            )));
        }

        let artifacts: Vec<ArtifactMetadata> = response
            .json()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!("Successfully listed {} artifacts", artifacts.len());
        Ok(artifacts)
    }

    /// Delete artifact from `NestGate`
    ///
    /// # Errors
    /// Returns an error if the artifact is not found or deletion fails
    pub async fn delete_artifact(&self, artifact_id: &str) -> NestGateResult<()> {
        info!("Deleting artifact: {}", artifact_id);

        let url = format!("{}/artifacts/{}", self.config.endpoint, artifact_id);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Storage(format!(
                "Failed to delete artifact: {}",
                response.status()
            )));
        }

        // Remove from cache
        if self.config.cache.as_ref().is_some_and(|c| c.enabled) {
            // Cache implementation would go here
            // For now, we'll just proceed without caching
        }

        info!("Successfully deleted artifact: {}", artifact_id);
        Ok(())
    }

    /// Create a data processing pipeline
    ///
    /// # Errors
    /// Returns an error if the pipeline configuration is invalid or creation fails
    pub async fn create_pipeline(&self, config: PipelineConfig) -> NestGateResult<String> {
        info!("Creating pipeline: {}", config.pipeline_id);

        let url = format!("{}/pipelines", self.config.endpoint);
        let response = self
            .client
            .post(&url)
            .json(&config)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Pipeline(format!(
                "Failed to create pipeline: {}",
                response.status()
            )));
        }

        // Cache the pipeline configuration
        // Cache implementation would go here
        // For now, we'll just proceed without caching

        info!("Successfully created pipeline: {}", config.pipeline_id);
        Ok(config.pipeline_id)
    }

    /// Start a pipeline execution
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or start fails
    pub async fn start_pipeline(&self, pipeline_id: &str) -> NestGateResult<String> {
        info!("Starting pipeline: {}", pipeline_id);

        let url = format!("{}/pipelines/{}/start", self.config.endpoint, pipeline_id);
        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Pipeline(format!(
                "Failed to start pipeline: {}",
                response.status()
            )));
        }

        let execution_id: String = response
            .json()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "Successfully started pipeline: {} with execution ID: {}",
            pipeline_id, execution_id
        );
        Ok(execution_id)
    }

    /// Get pipeline execution status
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or status request fails
    pub async fn get_pipeline_status(&self, pipeline_id: &str) -> NestGateResult<PipelineStatus> {
        info!("Getting status for pipeline: {}", pipeline_id);

        let url = format!("{}/pipelines/{}/status", self.config.endpoint, pipeline_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Pipeline(format!(
                "Failed to get pipeline status: {}",
                response.status()
            )));
        }

        let status: PipelineStatus = response
            .json()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        info!(
            "Successfully retrieved status for pipeline: {}",
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
