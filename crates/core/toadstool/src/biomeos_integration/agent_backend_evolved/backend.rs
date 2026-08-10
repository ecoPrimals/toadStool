// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`AgentBackend`] implementation using capability discovery and JSON-RPC-style calls.

use serde_json::json;
use std::sync::Arc;
use toadstool_common::capability_provider::{CapabilityError, CapabilityProvider};
use toadstool_common::primal_identity::Capability;
use std::sync::RwLock;

use super::error::{AgentBackendError, Result};
use super::types::{AgentInfo, DeployAgentRequest, LoadModelRequest, ModelInfo};

/// Agent backend with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs AI agent deployment
/// 2. **Runtime discovery**: Finds provider by capability
/// 3. **Proper errors**: No `unwrap()`, all errors handled
/// 4. **Agnostic**: Doesn't care which primal provides agents
pub struct AgentBackend {
    /// Agent provider (discovered at runtime)
    pub(super) provider: Arc<RwLock<Option<CapabilityProvider>>>,
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
        let mut provider_lock = self.provider.write().unwrap_or_else(|e| e.into_inner());

        if provider_lock.is_none() {
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
    /// Before: `call_rpc("/primal/intelligence", "ai.deploy_agent", ...)`
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
        let provider_lock = self.provider.read().unwrap_or_else(|e| e.into_inner());
        provider_lock.as_ref().map(|p| p.service_name().to_string())
    }
}

impl Default for AgentBackend {
    fn default() -> Self {
        Self::new()
    }
}
