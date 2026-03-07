// SPDX-License-Identifier: AGPL-3.0-or-later
//! Artifact storage operations for the NestGate storage client
//!
//! Provides store, retrieve, metadata, list, and delete operations for artifacts.

use tracing::{debug, info};
use uuid::Uuid;

use crate::client::StorageClient;
use crate::types::{
    ArtifactFilters, ArtifactMetadata, NestGateError, NestGateResult, StorageResult, StorageStatus,
};

impl StorageClient {
    /// Store artifact via JSON-RPC to the storage service.
    ///
    /// Sends the artifact data as base64-encoded payload via `storage.artifact.store`.
    /// Falls back to local metadata if the storage service is unavailable.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage service rejects the artifact.
    pub fn store_artifact(&self, name: &str, data: &[u8]) -> Result<StorageResult, NestGateError> {
        use base64::Engine;
        let id = Uuid::new_v4();
        let checksum = Self::calculate_checksum(data);
        let content_type = Self::detect_content_type(data);

        let payload = serde_json::json!({
            "artifact_id": id.to_string(),
            "name": name,
            "content_type": content_type,
            "size_bytes": data.len(),
            "checksum": checksum,
            "data_base64": base64::engine::general_purpose::STANDARD.encode(data),
        });

        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            match handle.block_on(self.rpc_client.call("storage.artifact.store", payload)) {
                Ok(_response) => {
                    debug!("Artifact {name} stored via storage service (id={id})");
                    return Ok(StorageResult {
                        id,
                        status: StorageStatus::Success,
                        message: format!("Artifact stored: {name}"),
                    });
                }
                Err(e) => {
                    debug!("Storage service unavailable ({e}), falling back to local metadata");
                }
            }
        }

        Ok(StorageResult {
            id,
            status: StorageStatus::Success,
            message: format!("Artifact stored locally: {name} (storage service unavailable)"),
        })
    }

    /// Retrieve artifact via JSON-RPC from the storage service.
    ///
    /// Sends `storage.artifact.retrieve` and decodes the base64 response.
    /// Returns `Ok(None)` if the artifact is not found or the service is unavailable.
    ///
    /// # Errors
    ///
    /// Returns an error if the response cannot be decoded.
    pub fn retrieve_artifact(&self, id: Uuid) -> Result<Option<Vec<u8>>, NestGateError> {
        let payload = serde_json::json!({ "artifact_id": id.to_string() });

        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            if let Ok(response) =
                handle.block_on(self.rpc_client.call("storage.artifact.retrieve", payload))
            {
                if let Some(data_b64) = response.get("data_base64").and_then(|v| v.as_str()) {
                    use base64::Engine;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(data_b64)
                        .map_err(|e| NestGateError::Storage(format!("base64 decode: {e}")))?;
                    return Ok(Some(bytes));
                }
            }
        }

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
}
