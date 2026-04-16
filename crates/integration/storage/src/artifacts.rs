// SPDX-License-Identifier: AGPL-3.0-or-later
//! Artifact storage operations for the storage service client
//!
//! Provides store, retrieve, metadata, list, and delete operations for artifacts.

use tracing::{debug, info};
use uuid::Uuid;

use crate::client::StorageClient;
use crate::types::{
    ArtifactFilters, ArtifactMetadata, StorageError, StorageResult, StorageServiceResult,
    StorageStatus,
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
    pub async fn store_artifact(
        &self,
        name: &str,
        data: &[u8],
    ) -> Result<StorageResult, StorageError> {
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

        match self
            .rpc_client
            .call("storage.artifact.store", payload)
            .await
        {
            Ok(_response) => {
                debug!("Artifact {name} stored via storage service (id={id})");
                Ok(StorageResult {
                    id,
                    status: StorageStatus::Success,
                    message: format!("Artifact stored: {name}"),
                })
            }
            Err(e) => {
                debug!("Storage service unavailable ({e}), falling back to local metadata");
                Ok(StorageResult {
                    id,
                    status: StorageStatus::LocalOnly,
                    message: format!(
                        "Artifact not persisted remotely (local-only/deferred): {name} (storage service unavailable)"
                    ),
                })
            }
        }
    }

    /// Retrieve artifact via JSON-RPC from the storage service.
    ///
    /// Sends `storage.artifact.retrieve` and decodes the base64 response.
    /// Returns `Ok(None)` if the artifact is not found or the service is unavailable.
    ///
    /// # Errors
    ///
    /// Returns an error if the response cannot be decoded.
    pub async fn retrieve_artifact(&self, id: Uuid) -> Result<Option<Vec<u8>>, StorageError> {
        let payload = serde_json::json!({ "artifact_id": id.to_string() });

        match self
            .rpc_client
            .call("storage.artifact.retrieve", payload)
            .await
        {
            Ok(response) => {
                if let Some(data_b64) = response.get("data_base64").and_then(|v| v.as_str()) {
                    use base64::Engine;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(data_b64)
                        .map_err(|e| StorageError::Storage(format!("base64 decode: {e}")))?;
                    Ok(Some(bytes))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
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
    ) -> StorageServiceResult<ArtifactMetadata> {
        info!("Getting metadata for artifact: {}", artifact_id);

        // Modern async RPC call
        let metadata: ArtifactMetadata = self
            .rpc_client
            .call_typed(
                "storage.artifact.get_metadata",
                serde_json::json!({ "artifact_id": artifact_id }),
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

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
    ) -> StorageServiceResult<Vec<ArtifactMetadata>> {
        info!("Listing artifacts with filters: {:?}", filters);

        // Modern async RPC call with optional filters
        let artifacts: Vec<ArtifactMetadata> = self
            .rpc_client
            .call_typed(
                "storage.artifact.list",
                serde_json::json!({ "filters": filters }),
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

        info!("✅ Successfully listed {} artifacts", artifacts.len());
        Ok(artifacts)
    }

    /// Delete artifact from `Storage`
    ///
    /// # Errors
    /// Returns an error if the artifact is not found or deletion fails
    pub async fn delete_artifact(&self, artifact_id: &str) -> StorageServiceResult<()> {
        info!("Deleting artifact: {}", artifact_id);

        // Modern async RPC call
        let _response: serde_json::Value = self
            .rpc_client
            .call(
                "storage.artifact.delete",
                serde_json::json!({ "artifact_id": artifact_id }),
            )
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

        info!("✅ Successfully deleted artifact: {}", artifact_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::client::StorageClient;
    use crate::config::StorageConfig;
    fn client_without_live_socket() -> StorageClient {
        StorageClient::new_for_testing(
            StorageConfig::default(),
            "toadstool-artifacts-unit-test-service".to_string(),
        )
    }

    #[tokio::test]
    async fn store_artifact_when_rpc_unavailable_returns_local_only_fallback() {
        let client = client_without_live_socket();
        let result = client.store_artifact("unit.bin", b"payload").await.unwrap();
        assert!(matches!(
            result.status,
            crate::types::StorageStatus::LocalOnly
        ));
        assert!(
            result.message.contains("local-only")
                || result.message.contains("deferred")
                || result.message.contains("unavailable"),
            "unexpected message: {}",
            result.message
        );
    }

    #[tokio::test]
    async fn retrieve_artifact_when_rpc_unavailable_returns_none() {
        let client = client_without_live_socket();
        let out = client.retrieve_artifact(Uuid::nil()).await.unwrap();
        assert!(out.is_none());
    }
}
