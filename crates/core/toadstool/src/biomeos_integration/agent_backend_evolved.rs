// SPDX-License-Identifier: AGPL-3.0-or-later
// agent_backend_evolved.rs - Capability-based agent deployment backend
//
// DEEP DEBT EVOLUTION: Discovers AI/agent providers by capability, not by name.
// Doesn't know or care if it's "squirrel" - just asks "Who can deploy agents?"
//
// Migration from: agent_backend.rs (hardcoded "squirrel")
// Evolution: Capability-based discovery, proper error handling, zero unwrap()

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use toadstool_common::capability_provider::{CapabilityError, CapabilityProvider};
use toadstool_common::primal_identity::Capability;
use tokio::sync::RwLock;

/// Errors for agent backend
#[derive(Debug, thiserror::Error)]
pub enum AgentBackendError {
    #[error("AI agent provider not found")]
    NoAgentProvider,

    #[error("Agent deployment failed: {0}")]
    DeploymentFailed(String),

    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    #[error("Agent scaling failed: {0}")]
    ScalingFailed(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Agent termination failed: {0}")]
    TerminationFailed(String),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AgentBackendError>;

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub status: AgentStatus,
    pub replicas: u32,
    pub capabilities: Vec<String>,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Deploying,
    Running,
    Scaling,
    Stopped,
    Failed,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub size_bytes: u64,
    pub status: ModelStatus,
}

/// Model status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Loading,
    Ready,
    Unloading,
    Error,
}

/// Agent deployment request
#[derive(Debug, Serialize)]
pub struct DeployAgentRequest {
    pub name: String,
    pub model: String,
    pub replicas: u32,
    pub capabilities: Vec<String>,
}

/// Model load request
#[derive(Debug, Serialize)]
pub struct LoadModelRequest {
    pub name: String,
    pub model_type: String,
    pub source: String,
}

/// Agent backend with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs AI agent deployment
/// 2. **Runtime discovery**: Finds provider by capability
/// 3. **Proper errors**: No unwrap(), all errors handled
/// 4. **Agnostic**: Doesn't care which primal provides agents
pub struct AgentBackend {
    /// Agent provider (discovered at runtime)
    provider: Arc<RwLock<Option<CapabilityProvider>>>,
}

impl AgentBackend {
    /// Create new agent backend
    pub fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or discover agent provider
    ///
    /// Discovers by capability: "Who can deploy AI agents?"
    async fn get_provider(&self) -> Result<CapabilityProvider> {
        let mut provider_lock = self.provider.write().await;

        if provider_lock.is_none() {
            // Use Compute capability (AI agents run on compute infrastructure)
            use toadstool_common::primal_identity::ComputeCapability;
            let capability = Capability::Compute(ComputeCapability::NativeExecution);

            let discovered =
                CapabilityProvider::discover(capability)
                    .await
                    .map_err(|e| match e {
                        CapabilityError::NoProviderFound(_) => AgentBackendError::NoAgentProvider,
                        other => AgentBackendError::Capability(other),
                    })?;

            *provider_lock = Some(discovered);
        }

        provider_lock
            .as_ref()
            .ok_or(AgentBackendError::NoAgentProvider)
            .cloned()
    }

    /// Deploy an AI agent
    ///
    /// # Deep Debt Evolution
    ///
    /// Before: `call_rpc("/primal/squirrel", "squirrel.deploy_agent", ...)`
    /// After: `provider.call("ai.deploy_agent", ...)`
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable or deployment fails
    pub async fn deploy_agent(&self, request: DeployAgentRequest) -> Result<AgentInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "name": request.name,
            "model": request.model,
            "replicas": request.replicas,
            "capabilities": request.capabilities,
        });

        let response = provider
            .call("ai.deploy_agent", params)
            .await
            .map_err(|e| AgentBackendError::DeploymentFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(AgentBackendError::Json)
    }

    /// Load an AI model
    ///
    /// # Errors
    ///
    /// Returns error if model loading fails
    pub async fn load_model(&self, request: LoadModelRequest) -> Result<ModelInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "name": request.name,
            "model_type": request.model_type,
            "source": request.source,
        });

        let response = provider
            .call("ai.load_model", params)
            .await
            .map_err(|e| AgentBackendError::ModelLoadFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(AgentBackendError::Json)
    }

    /// Scale an agent (change replica count)
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist or scaling fails
    pub async fn scale_agent(&self, agent_id: &str, replicas: u32) -> Result<AgentInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "agent_id": agent_id,
            "replicas": replicas,
        });

        let response = provider
            .call("ai.scale_agent", params)
            .await
            .map_err(|e| AgentBackendError::ScalingFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(AgentBackendError::Json)
    }

    /// Stop an agent
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist
    pub async fn stop_agent(&self, agent_id: &str) -> Result<()> {
        let provider = self.get_provider().await?;

        let params = json!({
            "agent_id": agent_id,
        });

        provider
            .call("ai.stop_agent", params)
            .await
            .map_err(|e| AgentBackendError::TerminationFailed(e.to_string()))?;

        Ok(())
    }

    /// Remove an agent completely
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist
    pub async fn remove_agent(&self, agent_id: &str) -> Result<()> {
        let provider = self.get_provider().await?;

        let params = json!({
            "agent_id": agent_id,
        });

        provider
            .call("ai.remove_agent", params)
            .await
            .map_err(|e| AgentBackendError::TerminationFailed(e.to_string()))?;

        Ok(())
    }

    /// Get agent status
    ///
    /// # Errors
    ///
    /// Returns error if agent not found
    pub async fn get_agent_status(&self, agent_id: &str) -> Result<AgentInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "agent_id": agent_id,
        });

        let response = provider
            .call("ai.get_agent_status", params)
            .await
            .map_err(|_| AgentBackendError::AgentNotFound(agent_id.to_string()))?;

        serde_json::from_value(response).map_err(AgentBackendError::Json)
    }

    /// List all deployed agents
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        let provider = self.get_provider().await?;

        let response = provider
            .call("ai.list_agents", json!({}))
            .await
            .map_err(|e| {
                AgentBackendError::Capability(CapabilityError::RpcFailed(e.to_string()))
            })?;

        let agents = response["agents"].as_array().ok_or_else(|| {
            AgentBackendError::Capability(CapabilityError::InvalidResponse(
                "No agents array in response".into(),
            ))
        })?;

        agents
            .iter()
            .map(|a| serde_json::from_value(a.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(AgentBackendError::Json)
    }

    /// List available models
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let provider = self.get_provider().await?;

        let response = provider
            .call("ai.list_models", json!({}))
            .await
            .map_err(|e| {
                AgentBackendError::Capability(CapabilityError::RpcFailed(e.to_string()))
            })?;

        let models = response["models"].as_array().ok_or_else(|| {
            AgentBackendError::Capability(CapabilityError::InvalidResponse(
                "No models array in response".into(),
            ))
        })?;

        models
            .iter()
            .map(|m| serde_json::from_value(m.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(AgentBackendError::Json)
    }

    /// Unload a model
    ///
    /// # Errors
    ///
    /// Returns error if model doesn't exist
    pub async fn unload_model(&self, model_id: &str) -> Result<()> {
        let provider = self.get_provider().await?;

        let params = json!({
            "model_id": model_id,
        });

        provider
            .call("ai.unload_model", params)
            .await
            .map_err(|_| AgentBackendError::ModelNotFound(model_id.to_string()))?;

        Ok(())
    }

    /// Check if agent provider is available
    pub async fn is_available(&self) -> bool {
        self.get_provider().await.is_ok()
    }

    /// Get provider info (for debugging only!)
    pub async fn provider_info(&self) -> Option<String> {
        let provider_lock = self.provider.read().await;
        provider_lock.as_ref().map(|p| p.service_name().to_string())
    }
}

impl Default for AgentBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_backend_creation() {
        let backend = AgentBackend::new();
        let provider_lock = backend.provider.read().await;
        assert!(provider_lock.is_none());
    }

    #[test]
    fn test_agent_backend_default() {
        let backend = AgentBackend::default();
        assert_eq!(
            std::mem::size_of_val(&backend),
            std::mem::size_of::<AgentBackend>()
        );
    }

    #[test]
    fn test_agent_status_enum() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_ne!(AgentStatus::Running, AgentStatus::Stopped);
    }

    #[test]
    fn test_agent_status_all_variants() {
        let _ = AgentStatus::Deploying;
        let _ = AgentStatus::Running;
        let _ = AgentStatus::Scaling;
        let _ = AgentStatus::Stopped;
        let _ = AgentStatus::Failed;
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Running;
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json, "running");
        let parsed: AgentStatus = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, AgentStatus::Running);
    }

    #[test]
    fn test_model_status_all_variants() {
        let _ = ModelStatus::Loading;
        let _ = ModelStatus::Ready;
        let _ = ModelStatus::Unloading;
        let _ = ModelStatus::Error;
    }

    #[test]
    fn test_model_status_serialization() {
        let status = ModelStatus::Ready;
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json, "ready");
    }

    #[test]
    fn test_agent_info_constructor_and_serialization() {
        let info = AgentInfo {
            id: "agent-1".to_string(),
            name: "test-agent".to_string(),
            model: "gpt-4".to_string(),
            status: AgentStatus::Running,
            replicas: 2,
            capabilities: vec!["inference".to_string(), "embedding".to_string()],
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "agent-1");
        assert_eq!(json["name"], "test-agent");
        assert_eq!(json["model"], "gpt-4");
        assert_eq!(json["replicas"], 2);
        let deserialized: AgentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, info.id);
        assert_eq!(deserialized.replicas, 2);
    }

    #[test]
    fn test_model_info_constructor_and_serialization() {
        let info = ModelInfo {
            id: "model-1".to_string(),
            name: "gpt-4".to_string(),
            model_type: "transformer".to_string(),
            size_bytes: 1_000_000_000,
            status: ModelStatus::Ready,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "model-1");
        assert_eq!(json["size_bytes"], 1_000_000_000);
    }

    #[test]
    fn test_deploy_agent_request_constructor_and_serialization() {
        let req = DeployAgentRequest {
            name: "deploy-test".to_string(),
            model: "gpt-4".to_string(),
            replicas: 3,
            capabilities: vec!["inference".to_string()],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["name"], "deploy-test");
        assert_eq!(json["replicas"], 3);
    }

    #[test]
    fn test_load_model_request_constructor_and_serialization() {
        let req = LoadModelRequest {
            name: "gpt-4".to_string(),
            model_type: "transformer".to_string(),
            source: "s3://models/gpt4".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["name"], "gpt-4");
        assert_eq!(json["source"], "s3://models/gpt4");
    }

    #[test]
    fn test_error_messages() {
        let err = AgentBackendError::NoAgentProvider;
        assert!(err.to_string().contains("AI agent provider not found"));

        let err = AgentBackendError::AgentNotFound("test-agent".into());
        assert!(err.to_string().contains("test-agent"));

        let err = AgentBackendError::DeploymentFailed("failed".into());
        assert!(err.to_string().contains("Agent deployment failed"));

        let err = AgentBackendError::ModelLoadFailed("load err".into());
        assert!(err.to_string().contains("Model loading failed"));

        let err = AgentBackendError::ScalingFailed("scale err".into());
        assert!(err.to_string().contains("Agent scaling failed"));

        let err = AgentBackendError::ModelNotFound("m1".into());
        assert!(err.to_string().contains("Model not found"));

        let err = AgentBackendError::TerminationFailed("term err".into());
        assert!(err.to_string().contains("Agent termination failed"));
    }

    #[test]
    fn test_agent_info_clone() {
        let info = AgentInfo {
            id: "x".to_string(),
            name: "n".to_string(),
            model: "m".to_string(),
            status: AgentStatus::Stopped,
            replicas: 1,
            capabilities: vec![],
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
    }

    #[test]
    fn test_model_info_clone() {
        let info = ModelInfo {
            id: "m1".to_string(),
            name: "n".to_string(),
            model_type: "t".to_string(),
            size_bytes: 100,
            status: ModelStatus::Error,
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
    }

    // ── DEEP tests: error paths, no-provider, capability conversion ─────

    #[tokio::test]
    async fn test_is_available_returns_false_when_no_provider() {
        let backend = AgentBackend::new();
        let available = backend.is_available().await;
        assert!(!available, "No agent provider configured => not available");
    }

    #[tokio::test]
    async fn test_provider_info_returns_none_when_no_provider() {
        let backend = AgentBackend::new();
        let info = backend.provider_info().await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_deploy_agent_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let req = DeployAgentRequest {
            name: "test".to_string(),
            model: "gpt-4".to_string(),
            replicas: 1,
            capabilities: vec![],
        };
        let result = backend.deploy_agent(req).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                AgentBackendError::NoAgentProvider
                    | AgentBackendError::Capability(_)
                    | AgentBackendError::DeploymentFailed(_)
            ),
            "expected provider or deployment error, got {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_load_model_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let req = LoadModelRequest {
            name: "m".to_string(),
            model_type: "transformer".to_string(),
            source: "s3://x".to_string(),
        };
        let result = backend.load_model(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scale_agent_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let result = backend.scale_agent("agent-1", 2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_agent_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let result = backend.stop_agent("agent-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_agent_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let result = backend.remove_agent("agent-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_agent_status_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let result = backend.get_agent_status("agent-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_agents_returns_no_provider_error() {
        let backend = AgentBackend::new();
        let result = backend.list_agents().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_models_returns_no_provider_error() {
        let backend = AgentBackend::new();
        let result = backend.list_models().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unload_model_returns_error_without_provider() {
        let backend = AgentBackend::new();
        let result = backend.unload_model("model-1").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_backend_error_capability_conversion() {
        use toadstool_common::capability_provider::CapabilityError;
        use toadstool_common::primal_identity::ComputeCapability;
        let cap_err = CapabilityError::NoProviderFound(Capability::Compute(
            ComputeCapability::NativeExecution,
        ));
        let agent_err: AgentBackendError = cap_err.into();
        assert!(agent_err.to_string().contains("provider"));
    }

    #[test]
    fn test_agent_backend_error_json_conversion() {
        let json_err = serde_json::from_str::<AgentInfo>("not valid json").unwrap_err();
        let agent_err: AgentBackendError = json_err.into();
        assert!(agent_err.to_string().contains("json") || agent_err.to_string().contains("JSON"));
    }

    #[test]
    fn test_agent_status_serde_all_variants() {
        for s in ["deploying", "running", "scaling", "stopped", "failed"] {
            let parsed: AgentStatus = serde_json::from_str(&format!("\"{}\"", s)).unwrap();
            let _ = serde_json::to_string(&parsed).unwrap();
        }
    }

    #[test]
    fn test_model_status_serde_all_variants() {
        for s in ["loading", "ready", "unloading", "error"] {
            let parsed: ModelStatus = serde_json::from_str(&format!("\"{}\"", s)).unwrap();
            let _ = serde_json::to_string(&parsed).unwrap();
        }
    }
}
