// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`AgentDeploymentManager`] and discovery helpers using the pluggable agent backend trait.

use std::sync::Arc;

use super::super::agent_backend::{AgentBackend, AgentBackendDispatch};
use super::super::types::{AgentConfig, ModelConfig};
use super::config::AgentDeploymentConfig;
use crate::ToadStoolResult;
use tracing::warn;

/// Agent deployment manager for the intelligence / ML agent service.
///
/// Uses dependency injection via the `AgentBackend` trait for flexibility.
/// No conditional compilation or feature flags — the backend determines behavior.
pub struct AgentDeploymentManager {
    /// Configuration
    _config: AgentDeploymentConfig,
    /// Pluggable agent backend (intelligence service, in-memory, etc.)
    backend: Arc<AgentBackendDispatch>,
}

impl AgentDeploymentManager {
    /// Create a new agent deployment manager with custom backend
    #[must_use]
    pub fn new(config: AgentDeploymentConfig, backend: Arc<AgentBackendDispatch>) -> Self {
        Self {
            _config: config,
            backend,
        }
    }

    /// Discover and create agent manager via capability-based discovery
    ///
    /// This is the preferred method for creating an `AgentDeploymentManager`.
    /// It discovers an intelligence / ML service (or another AI provider) at runtime.
    ///
    /// # Discovery Order
    ///
    /// 1. Environment: capability-domain hints via [`toadstool_common::primal_sockets::SocketPathEnv`]
    ///    (`TOADSTOOL_AI_ENDPOINT`, `AI_PROCESSING_ENDPOINT`, legacy `SQUIRREL_ENDPOINT`, …)
    /// 2. mDNS/local network discovery for "ai-orchestration" or "storage" capability
    /// 3. Unix socket discovery at standard paths
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let manager = AgentDeploymentManager::discover().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no AI provider can be configured.
    pub async fn discover() -> crate::ToadStoolResult<Self> {
        Self::discover_with_config(AgentDeploymentConfig::default()).await
    }

    /// Discover AI provider with custom configuration
    ///
    /// **EVOLVED**: Uses capability-based discovery (no hardcoded primal names).
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no AI provider can be configured.
    pub async fn discover_with_config(
        config: AgentDeploymentConfig,
    ) -> crate::ToadStoolResult<Self> {
        // Priority 1: Try capability-based discovery first (Deep Debt compliant)
        match Self::with_ml_service(config.clone()).await {
            Ok(manager) => {
                tracing::info!("Discovered ML service via capability-based discovery");
                return Ok(manager);
            }
            Err(e) => {
                tracing::debug!("Capability discovery failed: {}, trying fallbacks", e);
            }
        }

        // Priority 2: Capability-domain / legacy routing endpoint hints (see SocketPathEnv)
        let socket_env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
        let has_hint = socket_env.routing_connection_hint.is_some();
        let has_config = !config.ai_processing_endpoint.is_empty();

        if has_hint || has_config {
            let source = if has_hint { "environment" } else { "config" };
            tracing::info!("Discovered ML service via {source}, connecting async");
            return Self::with_ml_service(config).await;
        }

        Err(crate::ToadStoolError::configuration(
            "No AI provider discovered. Set TOADSTOOL_AI_ENDPOINT / AI_PROCESSING_ENDPOINT (see \
             SocketPathEnv routing hints); configure ai_processing_endpoint in the agent deployment \
             config; or ensure an AI/orchestration service is reachable via capability discovery.",
        ))
    }

    /// Create a new manager with capability-based ML service discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers ML service by capability, not name.
    ///
    /// # Errors
    ///
    /// Returns an error if ML service discovery fails or the backend cannot be initialized.
    pub async fn with_ml_service(config: AgentDeploymentConfig) -> crate::ToadStoolResult<Self> {
        let backend = super::super::agent_backend::IntelligenceBackend::new_async(
            config.model_registry.clone(),
            config.agent_runtime.clone(),
            config.mcp_enabled,
        )
        .await?;
        Ok(Self {
            _config: config,
            backend: Arc::new(AgentBackendDispatch::Intelligence(backend)),
        })
    }

    /// Create a new manager with in-memory test backend
    #[must_use]
    #[cfg(any(test, feature = "test-mocks"))]
    pub fn with_inmemory(config: AgentDeploymentConfig) -> Self {
        let backend = super::super::agent_backend::InMemoryAgentBackend::new();
        Self {
            _config: config,
            backend: Arc::new(AgentBackendDispatch::InMemory(backend)),
        }
    }

    /// Initialize connection to the intelligence / ML backend (or test backend).
    ///
    /// # Errors
    ///
    /// Returns an error if the backend connection cannot be established.
    pub async fn initialize_intelligence_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// Deploy an AI agent from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if deployment fails, configuration is invalid, or backend is unavailable.
    pub async fn deploy_agent(
        &mut self,
        agent_config: &AgentConfig,
    ) -> ToadStoolResult<super::AgentInfo> {
        self.backend.deploy_agent(agent_config).await
    }

    /// Load a model for agent use
    ///
    /// # Errors
    ///
    /// Returns an error if model loading fails or configuration is invalid.
    pub async fn load_model(
        &mut self,
        model_config: &ModelConfig,
    ) -> ToadStoolResult<super::ModelInfo> {
        self.backend.load_model(model_config).await
    }

    /// Scale an agent to specified replica count
    ///
    /// # Errors
    ///
    /// Returns an error if scaling fails or the agent does not exist.
    pub async fn scale_agent(&mut self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        self.backend.scale_agent(agent_name, replicas).await
    }

    /// Stop an agent
    ///
    /// # Errors
    ///
    /// Returns an error if the agent cannot be stopped or does not exist.
    pub async fn stop_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.stop_agent(agent_name).await
    }

    /// Remove an agent
    ///
    /// # Errors
    ///
    /// Returns an error if removal fails or the agent does not exist.
    pub async fn remove_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.remove_agent(agent_name).await
    }

    /// Get agent status (now properly async!)
    ///
    /// # Errors
    ///
    /// Returns an error if the agent does not exist or backend is unavailable.
    pub async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<super::AgentStatus> {
        self.backend.get_agent_status(agent_name).await
    }

    /// List all deployed agents (now properly async!)
    pub async fn list_agents(&self) -> Vec<super::AgentInfo> {
        self.backend.list_agents().await.unwrap_or_else(|e| {
            warn!(error = %e, "list_agents: backend call failed; returning empty list");
            Vec::new()
        })
    }

    /// List all loaded models (now properly async!)
    pub async fn list_models(&self) -> Vec<super::ModelInfo> {
        self.backend.list_models().await.unwrap_or_else(|e| {
            warn!(error = %e, "list_models: backend call failed; returning empty list");
            Vec::new()
        })
    }

    /// Get agent resource usage (now properly async!)
    ///
    /// # Errors
    ///
    /// Returns an error if the agent does not exist or resource data cannot be retrieved.
    pub async fn get_agent_resources(
        &self,
        agent_name: &str,
    ) -> ToadStoolResult<super::AgentResourceUsage> {
        self.backend.get_agent_resources(agent_name).await
    }

    /// Unload a model
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be unloaded or does not exist.
    pub async fn unload_model(&mut self, model_name: &str) -> ToadStoolResult<()> {
        self.backend.unload_model(model_name).await
    }

    /// Health check for agent manager
    ///
    /// # Errors
    ///
    /// Returns an error if the backend health check fails.
    pub async fn health_check(&self) -> ToadStoolResult<()> {
        self.backend.health_check().await
    }
}
