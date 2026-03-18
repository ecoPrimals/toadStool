// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::execution::{ExecutionOutput, ExecutionStatus, RuntimeType};
use crate::universal::UniversalJobType;
use crate::{ExecutionResponse, ToadStoolError, ToadStoolResult, UniversalJob};
use std::time::Duration;

/// `BiomeOS` integration for the OS layer
pub struct BiomeOSIntegration {
    orchestrator: BiomeOrchestrator,
}

impl BiomeOSIntegration {
    pub async fn new() -> ToadStoolResult<Self> {
        let orchestrator = BiomeOrchestrator::new().await?;
        Ok(Self { orchestrator })
    }

    pub async fn execute_deployment(
        &self,
        job: UniversalJob,
    ) -> ToadStoolResult<ExecutionResponse> {
        self.orchestrator.execute_deployment(job).await
    }
}

/// `BiomeOS` orchestrator for team-isolated deployments
pub struct BiomeOrchestrator {
    /// biomeOS integration config
    #[allow(dead_code, reason = "stored for future reconfiguration")]
    config: BiomeOSConfig,
    /// Active biome deployments
    #[allow(dead_code, reason = "reserved for future deployment management")]
    active_deployments: Arc<RwLock<HashMap<String, BiomeDeployment>>>,
}

/// Configuration for biomeOS integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSConfig {
    /// Enable biomeOS integration
    pub enabled: bool,
    /// biomeOS endpoint
    pub endpoint: Option<String>,
    /// Team isolation settings
    pub team_isolation: bool,
    /// Resource quota enforcement
    pub resource_quota_enforcement: bool,
}

impl Default for BiomeOSConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: None,
            team_isolation: true,
            resource_quota_enforcement: true,
        }
    }
}

/// Biome deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDeployment {
    pub deployment_id: String,
    pub team_id: String,
    pub biome_manifest: serde_json::Value,
    pub status: BiomeDeploymentStatus,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated_at: SystemTime,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomeDeploymentStatus {
    Pending,
    Running,
    Stopped,
    Failed(String),
}

impl BiomeOrchestrator {
    /// Create a new biome orchestrator with default config
    #[allow(clippy::unused_async)] // API consistency with with_config
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(BiomeOSConfig::default()).await
    }

    /// Create a new biome orchestrator with custom config (e.g. for testing)
    #[allow(clippy::unused_async)] // API consistency with trait/async ecosystem
    pub async fn with_config(config: BiomeOSConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            config,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Initialize the orchestrator
    #[allow(clippy::unused_async)] // API consistency with DeploymentLayer trait
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        tracing::info!("Initializing biome orchestrator");
        Ok(())
    }

    /// Execute a deployment
    ///
    /// Validates the deployment configuration and returns a meaningful response.
    /// If the full deployment pipeline (biomeOS endpoint) is not available,
    /// validation still runs and returns proper errors for invalid configurations.
    #[allow(clippy::unused_async)] // API consistency with DeploymentLayer trait
    pub async fn execute_deployment(
        &self,
        job: UniversalJob,
    ) -> ToadStoolResult<ExecutionResponse> {
        tracing::info!("Executing biome deployment for job: {:?}", job.id);

        // Validate job type - must be BiomeOS
        let (team_id, biome_manifest) = match &job.job_type {
            UniversalJobType::BiomeOS {
                team_id,
                biome_manifest,
            } => (team_id.clone(), biome_manifest.clone()),
            other => {
                return Err(ToadStoolError::validation(format!(
                    "BiomeOrchestrator expects BiomeOS job type, got: {other:?}"
                )));
            }
        };

        // Validate team_id
        if team_id.trim().is_empty() {
            return Err(ToadStoolError::validation(
                "BiomeOS deployment requires non-empty team_id".to_string(),
            ));
        }

        // Validate biome_manifest - must be an object with at least one key
        let manifest_obj = biome_manifest.as_object().ok_or_else(|| {
            ToadStoolError::validation(
                "BiomeOS deployment requires biome_manifest to be a JSON object".to_string(),
            )
        })?;
        if manifest_obj.is_empty() {
            return Err(ToadStoolError::validation(
                "BiomeOS deployment requires non-empty biome_manifest object".to_string(),
            ));
        }

        // Check if biomeOS integration is enabled and endpoint configured
        if !self.config.enabled || self.config.endpoint.is_none() {
            return Ok(ExecutionResponse {
                execution_id: job.id,
                status: ExecutionStatus::Failed {
                    error: std::borrow::Cow::Borrowed(
                        "BiomeOS integration not configured: set enabled=true and endpoint in config. \
                         Full deployment pipeline requires biomeOS service.",
                    ),
                },
                output: ExecutionOutput::default(),
                metrics: crate::RuntimeMetrics::default(),
                duration: Duration::ZERO,
                runtime_used: RuntimeType::from(
                    toadstool_common::interned_strings::runtime_types::BIOMEOS,
                ),
                warnings: vec![
                    "BiomeOS integration disabled - validation passed but deployment not executed"
                        .to_string(),
                ],
            });
        }

        // Deployment validated; return success response
        // Full pipeline would contact biomeOS endpoint here
        Ok(ExecutionResponse {
            execution_id: job.id,
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::ZERO,
            runtime_used: RuntimeType::from(
                toadstool_common::interned_strings::runtime_types::BIOMEOS,
            ),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::types::{NetworkLocation, PrimalContext, SecurityLevel};
    use crate::universal::{JobPriority, UniversalJobType};
    use std::collections::HashMap;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn sample_job() -> UniversalJob {
        UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::BiomeOS {
                biome_manifest: serde_json::json!({"team": "test"}),
                team_id: "team-1".to_string(),
            },
            priority: JobPriority::Normal,
            resources: crate::resources::ResourceRequirements::default(),
            timeout: None,
            created_at: SystemTime::now(),
            context: PrimalContext {
                user_id: "user".to_string(),
                device_id: "device".to_string(),
                session_id: "session".to_string(),
                network_location: NetworkLocation {
                    ip_address: "127.0.0.1".to_string(),
                    subnet: None,
                    network_id: None,
                    geo_location: None,
                },
                security_level: SecurityLevel::Standard,
                metadata: HashMap::new(),
            },
        }
    }

    #[tokio::test]
    async fn test_biome_config_default() {
        let config = BiomeOSConfig::default();
        assert!(!config.enabled);
        assert!(config.endpoint.is_none());
        assert!(config.team_isolation);
        assert!(config.resource_quota_enforcement);
    }

    #[tokio::test]
    async fn test_biome_config_custom() {
        let config = BiomeOSConfig {
            enabled: true,
            endpoint: Some("http://biome:8080".to_string()),
            team_isolation: false,
            resource_quota_enforcement: false,
        };
        assert!(config.enabled);
        assert_eq!(config.endpoint.as_deref(), Some("http://biome:8080"));
        assert!(!config.team_isolation);
        assert!(!config.resource_quota_enforcement);
    }

    #[tokio::test]
    async fn test_biome_deployment_status_variants() {
        let _pending = BiomeDeploymentStatus::Pending;
        let _running = BiomeDeploymentStatus::Running;
        let _stopped = BiomeDeploymentStatus::Stopped;
        let failed = BiomeDeploymentStatus::Failed("timeout".to_string());
        match &failed {
            BiomeDeploymentStatus::Failed(msg) => assert_eq!(msg, "timeout"),
            _ => panic!("expected Failed variant"),
        }
    }

    #[tokio::test]
    async fn test_biome_deployment_serde() {
        let deployment = BiomeDeployment {
            deployment_id: "dep-1".to_string(),
            team_id: "team-1".to_string(),
            biome_manifest: serde_json::json!({"key": "value"}),
            status: BiomeDeploymentStatus::Running,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };
        let json = serde_json::to_string(&deployment).expect("serialize");
        let restored: BiomeDeployment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.deployment_id, deployment.deployment_id);
        assert_eq!(restored.team_id, deployment.team_id);
    }

    #[tokio::test]
    async fn test_biome_orchestrator_new() {
        let orchestrator = BiomeOrchestrator::new()
            .await
            .expect("orchestrator creation");
        let result = orchestrator.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_biome_orchestrator_execute_deployment() {
        let orchestrator = BiomeOrchestrator::new()
            .await
            .expect("orchestrator creation");
        let job = sample_job();
        let response = orchestrator
            .execute_deployment(job)
            .await
            .expect("execute_deployment");
        // Default config has biomeOS disabled — real implementation returns Failed
        assert!(
            matches!(
                response.status,
                crate::execution::ExecutionStatus::Failed { .. }
            ),
            "BiomeOS should report failure when not configured: {:?}",
            response.status
        );
    }

    #[tokio::test]
    async fn test_biome_integration_new() {
        let integration = BiomeOSIntegration::new()
            .await
            .expect("integration creation");
        let job = sample_job();
        let response = integration
            .execute_deployment(job)
            .await
            .expect("execute_deployment");
        // Default config has biomeOS disabled — real implementation returns Failed
        assert!(
            matches!(
                response.status,
                crate::execution::ExecutionStatus::Failed { .. }
            ),
            "BiomeOS should report failure when not configured: {:?}",
            response.status
        );
    }

    #[tokio::test]
    async fn test_biome_deployment_status_serde() {
        let status = BiomeDeploymentStatus::Failed("error".to_string());
        let json = serde_json::to_value(&status).expect("serialize");
        let restored: BiomeDeploymentStatus = serde_json::from_value(json).expect("deserialize");
        match restored {
            BiomeDeploymentStatus::Failed(msg) => assert_eq!(msg, "error"),
            _ => panic!("expected Failed variant"),
        }
    }
}
