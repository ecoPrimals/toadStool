//! NestGate client implementation for artifact and storage management

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::config::NestGateConfig;
use crate::pipeline::{PipelineConfig, PipelineStatus};
use crate::types::{
    ArtifactFilters, ArtifactMetadata, ArtifactType, CachedArtifact, NestGateError, NestGateResult,
    StorageInfo, VersionInfo,
};

/// Main NestGate client for storage and pipeline operations
#[derive(Debug)]
pub struct NestGateClient {
    config: NestGateConfig,
    http_client: reqwest::Client,
    artifact_cache: Arc<RwLock<HashMap<String, CachedArtifact>>>,
    pipeline_cache: Arc<RwLock<HashMap<String, PipelineConfig>>>,
}

impl NestGateClient {
    /// Connect to NestGate server with default configuration
    pub async fn connect(endpoint: &str) -> NestGateResult<Self> {
        let config = NestGateConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create client with custom configuration
    pub async fn with_config(config: NestGateConfig) -> NestGateResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| NestGateError::Connection(e.to_string()))?;

        let client = Self {
            config,
            http_client,
            artifact_cache: Arc::new(RwLock::new(HashMap::new())),
            pipeline_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Perform initial health check
        client.health_check().await?;

        Ok(client)
    }

    /// Check NestGate server health
    pub async fn health_check(&self) -> NestGateResult<()> {
        let url = format!("{}/health", self.config.endpoint);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if response.status().is_success() {
            debug!("NestGate health check passed");
            Ok(())
        } else {
            error!("NestGate health check failed: {}", response.status());
            Err(NestGateError::Connection(format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }

    /// Store artifact data in NestGate
    pub async fn store_artifact(
        &self,
        artifact_id: &str,
        data: &[u8],
        artifact_type: ArtifactType,
    ) -> NestGateResult<ArtifactMetadata> {
        info!("Storing artifact: {} ({} bytes)", artifact_id, data.len());

        // Calculate checksum
        let checksum = self.calculate_checksum(data);

        // Create metadata
        let metadata = ArtifactMetadata {
            id: artifact_id.to_string(),
            artifact_type,
            content_type: self.detect_content_type(data),
            size_bytes: data.len() as u64,
            checksum: checksum.clone(),
            created_at: Utc::now(),
            last_accessed: None,
            tags: HashMap::new(),
            execution_id: None,
            storage_info: StorageInfo {
                node_id: "nestgate-node-1".to_string(),
                path: format!("/artifacts/{artifact_id}"),
                tier: self.config.storage_preferences.storage_tier.clone(),
                replicated: self.config.storage_preferences.replication_factor > 1,
                compression: self.config.storage_preferences.compression.clone(),
                encryption: self.config.storage_preferences.encryption.clone(),
            },
            version: Some(VersionInfo {
                version: 1,
                parent_version: None,
                timestamp: Utc::now(),
                description: Some("Initial version".to_string()),
                author: None,
            }),
        };

        // Store in NestGate (mock implementation)
        let url = format!("{}/artifacts/{}", self.config.endpoint, artifact_id);
        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .header("X-Checksum", &checksum)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Storage(format!(
                "Failed to store artifact: {}",
                response.status()
            )));
        }

        // Cache the artifact if caching is enabled
        if self.config.cache_config.enabled {
            let cached_artifact = CachedArtifact {
                metadata: metadata.clone(),
                data: Some(data.to_vec()),
                cached_at: Utc::now(),
                access_count: 0,
            };

            let mut cache = self.artifact_cache.write().await;
            cache.insert(artifact_id.to_string(), cached_artifact);
        }

        info!("Successfully stored artifact: {}", artifact_id);
        Ok(metadata)
    }

    /// Retrieve artifact data from NestGate
    pub async fn retrieve_artifact(&self, artifact_id: &str) -> NestGateResult<Vec<u8>> {
        info!("Retrieving artifact: {}", artifact_id);

        // Check cache first
        if self.config.cache_config.enabled {
            let mut cache = self.artifact_cache.write().await;
            if let Some(cached) = cache.get_mut(artifact_id) {
                cached.access_count += 1;
                if let Some(ref data) = cached.data {
                    debug!("Retrieved artifact from cache: {}", artifact_id);
                    return Ok(data.clone());
                }
            }
        }

        // Retrieve from NestGate
        let url = format!("{}/artifacts/{}", self.config.endpoint, artifact_id);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NestGateError::Storage(format!(
                "Failed to retrieve artifact: {}",
                response.status()
            )));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| NestGateError::Network(e.to_string()))?
            .to_vec();

        info!(
            "Successfully retrieved artifact: {} ({} bytes)",
            artifact_id,
            data.len()
        );
        Ok(data)
    }

    /// Get artifact metadata
    pub async fn get_artifact_metadata(
        &self,
        artifact_id: &str,
    ) -> NestGateResult<ArtifactMetadata> {
        info!("Getting metadata for artifact: {}", artifact_id);

        // Check cache first
        if self.config.cache_config.enabled {
            let cache = self.artifact_cache.read().await;
            if let Some(cached) = cache.get(artifact_id) {
                debug!("Retrieved metadata from cache: {}", artifact_id);
                return Ok(cached.metadata.clone());
            }
        }

        // Get from NestGate
        let url = format!(
            "{}/artifacts/{}/metadata",
            self.config.endpoint, artifact_id
        );
        let response = self
            .http_client
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

    /// List artifacts with optional filters
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
            .http_client
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

    /// Delete artifact from NestGate
    pub async fn delete_artifact(&self, artifact_id: &str) -> NestGateResult<()> {
        info!("Deleting artifact: {}", artifact_id);

        let url = format!("{}/artifacts/{}", self.config.endpoint, artifact_id);
        let response = self
            .http_client
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
        if self.config.cache_config.enabled {
            let mut cache = self.artifact_cache.write().await;
            cache.remove(artifact_id);
        }

        info!("Successfully deleted artifact: {}", artifact_id);
        Ok(())
    }

    /// Create a new data pipeline
    pub async fn create_pipeline(&self, config: PipelineConfig) -> NestGateResult<String> {
        info!("Creating pipeline: {}", config.pipeline_id);

        let url = format!("{}/pipelines", self.config.endpoint);
        let response = self
            .http_client
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
        let mut cache = self.pipeline_cache.write().await;
        cache.insert(config.pipeline_id.clone(), config.clone());

        info!("Successfully created pipeline: {}", config.pipeline_id);
        Ok(config.pipeline_id)
    }

    /// Start pipeline execution
    pub async fn start_pipeline(&self, pipeline_id: &str) -> NestGateResult<String> {
        info!("Starting pipeline: {}", pipeline_id);

        let url = format!("{}/pipelines/{}/start", self.config.endpoint, pipeline_id);
        let response = self
            .http_client
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
    pub async fn get_pipeline_status(&self, pipeline_id: &str) -> NestGateResult<PipelineStatus> {
        info!("Getting status for pipeline: {}", pipeline_id);

        let url = format!("{}/pipelines/{}/status", self.config.endpoint, pipeline_id);
        let response = self
            .http_client
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

    /// Calculate SHA-256 checksum of data
    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Detect content type based on data
    fn detect_content_type(&self, data: &[u8]) -> String {
        if data.starts_with(b"\x89PNG") {
            "image/png".to_string()
        } else if data.starts_with(b"\xFF\xD8\xFF") {
            "image/jpeg".to_string()
        } else if data.starts_with(b"GIF8") {
            "image/gif".to_string()
        } else if data.starts_with(b"\x00\x61\x73\x6D") {
            "application/wasm".to_string()
        } else if data.starts_with(b"PK\x03\x04") {
            "application/zip".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }

    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) {
        if !self.config.cache_config.enabled {
            return;
        }

        let mut cache = self.artifact_cache.write().await;
        let now = Utc::now();
        let ttl = self.config.cache_config.artifact_ttl;

        cache.retain(|_, cached| {
            now.signed_duration_since(cached.cached_at)
                < chrono::Duration::from_std(ttl).unwrap_or_default()
        });

        debug!("Cache cleanup completed, {} entries remaining", cache.len());
    }
}
