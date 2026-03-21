// SPDX-License-Identifier: AGPL-3.0-only
use crate::client::{PrimalClient, PrimalRequest};
use crate::error::{PrimalError, PrimalResult};
use crate::manifest::BiomeManifest;
use crate::types::primal::PrimalIntegration;
use std::collections::HashMap;
use tracing::info;

/// Orchestrator for managing primals
pub struct PrimalOrchestrator {
    primals: HashMap<String, Box<dyn PrimalIntegration>>,
    /// Optional deployment endpoint (Unix socket path) for biome.deploy JSON-RPC
    deployment_endpoint: Option<String>,
}

impl Default for PrimalOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimalOrchestrator {
    pub fn new() -> Self {
        Self {
            primals: HashMap::new(),
            deployment_endpoint: None,
        }
    }

    /// Set the deployment endpoint (Unix socket path) for biome.deploy requests
    #[must_use]
    pub fn with_deployment_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.deployment_endpoint = Some(endpoint.into());
        self
    }

    /// Validate the deployment request
    fn validate_manifest(manifest: &BiomeManifest) -> PrimalResult<()> {
        if manifest.name.is_empty() {
            return Err(PrimalError::Validation {
                message: "Biome manifest name cannot be empty".to_string(),
            });
        }
        if manifest.version.is_empty() {
            return Err(PrimalError::Validation {
                message: "Biome manifest version cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    pub async fn deploy_biome(&self, manifest: BiomeManifest) -> PrimalResult<String> {
        Self::validate_manifest(&manifest)?;

        let endpoint = self.deployment_endpoint.as_ref().ok_or_else(|| {
            PrimalError::Configuration {
                message: "Deployment endpoint not configured. Call with_deployment_endpoint() before deploy_biome.".to_string(),
            }
        })?;

        let client = PrimalClient::new(endpoint);

        let request = PrimalRequest {
            action: "biome.deploy".to_string(),
            payload: serde_json::to_value(&manifest).map_err(|e| PrimalError::Integration {
                primal: "biome".to_string(),
                message: format!("Failed to serialize manifest: {e}"),
            })?,
        };

        info!("Deploying biome: {} via {}", manifest.name, endpoint);

        let response = client.send_request(request).await?;

        if let Some(err) = &response.error {
            return Err(PrimalError::Integration {
                primal: "biome".to_string(),
                message: err.clone(),
            });
        }

        if !response.success {
            return Err(PrimalError::Integration {
                primal: "biome".to_string(),
                message: response
                    .data
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Deployment failed")
                    .to_string(),
            });
        }

        let deployment_id = response
            .data
            .get("deployment_id")
            .or_else(|| response.data.get("id"))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| format!("deployed-{}", manifest.name));

        Ok(deployment_id)
    }

    pub async fn register_primal(
        &mut self,
        primal_id: String,
        primal: Box<dyn PrimalIntegration>,
    ) -> PrimalResult<()> {
        self.primals.insert(primal_id, primal);
        Ok(())
    }

    pub async fn get_primal(&self, primal_id: &str) -> Option<&dyn PrimalIntegration> {
        self.primals.get(primal_id).map(|p| p.as_ref())
    }
}
