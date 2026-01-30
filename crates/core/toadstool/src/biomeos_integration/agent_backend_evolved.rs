// agent_backend_evolved.rs - Capability-based agent deployment backend
//
// DEEP DEBT EVOLUTION: Discovers AI/agent providers by capability, not by name.
// Doesn't know or care if it's "squirrel" - just asks "Who can deploy agents?"
//
// Migration from: agent_backend.rs (hardcoded "squirrel")
// Evolution: Capability-based discovery, proper error handling, zero unwrap()

use toadstool_common::capability_provider::{CapabilityProvider, CapabilityError};
use toadstool_common::primal_identity::Capability;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
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
            
            let discovered = CapabilityProvider::discover(capability)
                .await
                .map_err(|e| match e {
                    CapabilityError::NoProviderFound(_) => AgentBackendError::NoAgentProvider,
                    other => AgentBackendError::Capability(other),
                })?;
            
            *provider_lock = Some(discovered);
        }
        
        Ok(provider_lock.as_ref().unwrap().clone())
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
        
        let response = provider.call("ai.deploy_agent", params)
            .await
            .map_err(|e| AgentBackendError::DeploymentFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(AgentBackendError::Json)
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
        
        let response = provider.call("ai.load_model", params)
            .await
            .map_err(|e| AgentBackendError::ModelLoadFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(AgentBackendError::Json)
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
        
        let response = provider.call("ai.scale_agent", params)
            .await
            .map_err(|e| AgentBackendError::ScalingFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(AgentBackendError::Json)
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
        
        provider.call("ai.stop_agent", params)
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
        
        provider.call("ai.remove_agent", params)
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
        
        let response = provider.call("ai.get_agent_status", params)
            .await
            .map_err(|_| AgentBackendError::AgentNotFound(agent_id.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(AgentBackendError::Json)
    }
    
    /// List all deployed agents
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        let provider = self.get_provider().await?;
        
        let response = provider.call("ai.list_agents", json!({}))
            .await
            .map_err(|e| AgentBackendError::Capability(CapabilityError::RpcFailed(e.to_string())))?;
        
        let agents = response["agents"]
            .as_array()
            .ok_or_else(|| AgentBackendError::Capability(
                CapabilityError::InvalidResponse("No agents array in response".into())
            ))?;
        
        agents.iter()
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
        
        let response = provider.call("ai.list_models", json!({}))
            .await
            .map_err(|e| AgentBackendError::Capability(CapabilityError::RpcFailed(e.to_string())))?;
        
        let models = response["models"]
            .as_array()
            .ok_or_else(|| AgentBackendError::Capability(
                CapabilityError::InvalidResponse("No models array in response".into())
            ))?;
        
        models.iter()
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
        
        provider.call("ai.unload_model", params)
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
    fn test_agent_status_enum() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_ne!(AgentStatus::Running, AgentStatus::Stopped);
    }
    
    #[test]
    fn test_error_messages() {
        let err = AgentBackendError::NoAgentProvider;
        assert!(err.to_string().contains("AI agent provider not found"));
        
        let err = AgentBackendError::AgentNotFound("test-agent".into());
        assert!(err.to_string().contains("test-agent"));
    }
}
