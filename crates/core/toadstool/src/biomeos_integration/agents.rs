//! AI agent deployment and management via Squirrel
//!
//! This module handles AI agent deployment, model management, and integration
//! with Squirrel AI services using trait-based dependency injection.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::agent_backend::AgentBackend;
use super::types::{AgentConfig, ModelConfig};
use crate::ToadStoolResult;

// Re-export all types for backward compatibility
pub use super::agent_backend::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

/// Agent deployment manager for Squirrel integration
///
/// Uses dependency injection via the `AgentBackend` trait for flexibility.
/// No conditional compilation or feature flags - the backend determines behavior.
pub struct AgentDeploymentManager {
    /// Configuration
    _config: AgentDeploymentConfig,
    /// Pluggable agent backend (Squirrel, in-memory, etc.)
    backend: Arc<dyn AgentBackend>,
}

/// Configuration for agent deployment manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeploymentConfig {
    /// Squirrel endpoint URL
    pub squirrel_endpoint: String,
    /// Model registry type (local, huggingface, custom)
    pub model_registry: String,
    /// Agent runtime (container, process, lambda)
    pub agent_runtime: String,
    /// Enable MCP (Model Control Protocol)
    pub mcp_enabled: bool,
    /// Resource limits configuration
    pub resource_limits: serde_json::Map<String, serde_json::Value>,
}

impl AgentDeploymentManager {
    /// Create a new agent deployment manager with custom backend
    #[must_use]
    pub fn new(config: AgentDeploymentConfig, backend: Arc<dyn AgentBackend>) -> Self {
        Self {
            _config: config,
            backend,
        }
    }

    /// Create a new manager with Squirrel production backend
    #[must_use]
    pub fn with_squirrel(config: AgentDeploymentConfig) -> Self {
        let backend = super::agent_backend::SquirrelBackend::new(
            config.squirrel_endpoint.clone(),
            config.model_registry.clone(),
            config.agent_runtime.clone(),
            config.mcp_enabled,
        );
        Self {
            _config: config,
            backend: Arc::new(backend),
        }
    }

    /// Create a new manager with in-memory test backend
    #[must_use]
    pub fn with_inmemory(config: AgentDeploymentConfig) -> Self {
        let backend = super::agent_backend::InMemoryAgentBackend::new();
        Self {
            _config: config,
            backend: Arc::new(backend),
        }
    }

    /// Initialize connection to Squirrel (or test backend)
    pub async fn initialize_squirrel_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// Deploy an AI agent from configuration
    pub async fn deploy_agent(&mut self, agent_config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        self.backend.deploy_agent(agent_config).await
    }

    /// Load a model for agent use
    pub async fn load_model(&mut self, model_config: &ModelConfig) -> ToadStoolResult<ModelInfo> {
        self.backend.load_model(model_config).await
    }

    /// Scale an agent to specified replica count
    pub async fn scale_agent(&mut self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        self.backend.scale_agent(agent_name, replicas).await
    }

    /// Stop an agent
    pub async fn stop_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.stop_agent(agent_name).await
    }

    /// Remove an agent
    pub async fn remove_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.remove_agent(agent_name).await
    }

    /// Get agent status
    pub fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus> {
        // Create a blocking bridge since AgentBackend is async
        tokio::runtime::Handle::current().block_on(self.backend.get_agent_status(agent_name))
    }

    /// List all deployed agents
    #[must_use]
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        // Create a blocking bridge since AgentBackend is async
        tokio::runtime::Handle::current()
            .block_on(self.backend.list_agents())
            .unwrap_or_default()
    }

    /// List all loaded models
    #[must_use]
    pub fn list_models(&self) -> Vec<ModelInfo> {
        // Create a blocking bridge since AgentBackend is async
        tokio::runtime::Handle::current()
            .block_on(self.backend.list_models())
            .unwrap_or_default()
    }

    /// Get agent resource usage
    pub fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage> {
        // Create a blocking bridge since AgentBackend is async
        tokio::runtime::Handle::current().block_on(self.backend.get_agent_resources(agent_name))
    }

    /// Unload a model
    pub async fn unload_model(&mut self, model_name: &str) -> ToadStoolResult<()> {
        self.backend.unload_model(model_name).await
    }

    /// Health check for agent manager
    pub async fn health_check(&self) -> ToadStoolResult<()> {
        self.backend.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> AgentDeploymentConfig {
        AgentDeploymentConfig {
            squirrel_endpoint: "http://localhost:8080".to_string(),
            model_registry: "local".to_string(),
            agent_runtime: "container".to_string(),
            mcp_enabled: true,
            resource_limits: serde_json::Map::new(),
        }
    }

    #[tokio::test]
    async fn test_manager_with_inmemory_backend() {
        let config = test_config();
        let mut manager = AgentDeploymentManager::with_inmemory(config);

        let agent_config = AgentConfig {
            name: "test-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };

        let result = manager.deploy_agent(&agent_config).await;
        assert!(result.is_ok());

        let agent_info = result.unwrap();
        assert_eq!(agent_info.name, "test-agent");
        assert!(agent_info.agent_id.starts_with("test-agent-"));
        assert_eq!(agent_info.status, AgentStatus::Running);
    }
}
