// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pipeline operations for the storage service client
//!
//! Provides create, start, and status operations for data processing pipelines.

use tracing::info;

use crate::client::StorageClient;
use crate::pipeline::{PipelineConfig, PipelineStatus};
use crate::types::{StorageError, StorageServiceResult};

impl StorageClient {
    /// Create a data processing pipeline - Modern async
    ///
    /// **MODERN ASYNC**: Concurrent pipeline creation
    ///
    /// # Errors
    /// Returns an error if the pipeline configuration is invalid or creation fails
    pub async fn create_pipeline(&self, config: PipelineConfig) -> StorageServiceResult<String> {
        info!("Creating pipeline: {}", config.pipeline_id);

        // Modern async RPC call
        let pipeline_id: String = self
            .rpc_client
            .call_typed(
                "storage.pipeline.create",
                serde_json::to_value(&config).map_err(|e| StorageError::Pipeline(e.to_string()))?,
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

        info!("✅ Successfully created pipeline: {}", pipeline_id);
        Ok(pipeline_id)
    }

    /// Start a pipeline execution - Modern async
    ///
    /// **MODERN ASYNC**: Non-blocking pipeline start
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or start fails
    pub async fn start_pipeline(&self, pipeline_id: &str) -> StorageServiceResult<String> {
        info!("Starting pipeline: {}", pipeline_id);

        // Modern async RPC call
        let execution_id: String = self
            .rpc_client
            .call_typed(
                "storage.pipeline.start",
                serde_json::json!({ "pipeline_id": pipeline_id }),
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

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
    pub async fn get_pipeline_status(
        &self,
        pipeline_id: &str,
    ) -> StorageServiceResult<PipelineStatus> {
        info!("Getting status for pipeline: {}", pipeline_id);

        // Modern async RPC call
        let status: PipelineStatus = self
            .rpc_client
            .call_typed(
                "storage.pipeline.get_status",
                serde_json::json!({ "pipeline_id": pipeline_id }),
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

        info!(
            "✅ Successfully retrieved status for pipeline: {}",
            pipeline_id
        );
        Ok(status)
    }
}
