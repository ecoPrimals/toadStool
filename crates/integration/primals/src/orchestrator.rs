// SPDX-License-Identifier: AGPL-3.0-or-later
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
            .and_then(serde_json::Value::as_str)
            .map_or_else(
                || format!("deployed-{}", manifest.name),
                std::string::ToString::to_string,
            );

        Ok(deployment_id)
    }

    pub fn register_primal(
        &mut self,
        primal_id: String,
        primal: Box<dyn PrimalIntegration>,
    ) -> PrimalResult<()> {
        self.primals.insert(primal_id, primal);
        Ok(())
    }

    pub fn get_primal(&self, primal_id: &str) -> Option<&dyn PrimalIntegration> {
        self.primals.get(primal_id).map(std::convert::AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str, version: &str) -> BiomeManifest {
        BiomeManifest {
            name: name.to_string(),
            version: version.to_string(),
            description: Some("test".to_string()),
            primals: HashMap::new(),
            storage: None,
            agents: None,
            security: None,
            services: vec![],
            networking: None,
            resources: None,
            federation: None,
            health_checks: vec![],
        }
    }

    #[test]
    fn default_creates_empty_orchestrator() {
        let orch = PrimalOrchestrator::default();
        assert!(orch.primals.is_empty());
        assert!(orch.deployment_endpoint.is_none());
    }

    #[test]
    fn with_deployment_endpoint_sets_endpoint() {
        let orch = PrimalOrchestrator::new().with_deployment_endpoint("/tmp/deploy.sock");
        assert_eq!(
            orch.deployment_endpoint.as_deref(),
            Some("/tmp/deploy.sock")
        );
    }

    #[test]
    fn validate_manifest_rejects_empty_name() {
        let m = manifest("", "1.0");
        let err = PrimalOrchestrator::validate_manifest(&m).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn validate_manifest_rejects_empty_version() {
        let m = manifest("my-biome", "");
        let err = PrimalOrchestrator::validate_manifest(&m).unwrap_err();
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn validate_manifest_accepts_valid() {
        let m = manifest("my-biome", "1.0.0");
        assert!(PrimalOrchestrator::validate_manifest(&m).is_ok());
    }

    #[tokio::test]
    async fn deploy_biome_fails_without_endpoint() {
        let orch = PrimalOrchestrator::new();
        let m = manifest("test-biome", "1.0.0");
        let err = orch.deploy_biome(m).await.unwrap_err();
        assert!(err.to_string().contains("endpoint"));
    }

    #[tokio::test]
    async fn deploy_biome_validates_manifest_first() {
        let orch = PrimalOrchestrator::new().with_deployment_endpoint("/tmp/deploy.sock");
        let m = manifest("", "1.0.0");
        let err = orch.deploy_biome(m).await.unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[tokio::test]
    async fn register_and_get_primal() {
        use crate::mock_primal::MockPrimal;
        let mut orch = PrimalOrchestrator::new();
        let mock = MockPrimal {
            name: "test-primal".to_string(),
            should_fail: false,
        };
        orch.register_primal("test".to_string(), Box::new(mock))
            .unwrap();
        assert!(orch.get_primal("test").is_some());
        assert!(orch.get_primal("nonexistent").is_none());
    }
}
