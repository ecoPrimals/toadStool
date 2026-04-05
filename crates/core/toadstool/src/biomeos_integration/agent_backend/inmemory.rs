// SPDX-License-Identifier: AGPL-3.0-or-later
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

use super::super::types::{AgentConfig, ModelConfig};
use super::AgentBackend;
use super::types::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};
use crate::{ToadStoolError, ToadStoolResult};

/// In-memory test backend for testing without external dependencies
///
/// This is a proper test implementation, not a mock. It maintains full state
/// and implements the complete backend interface correctly for testing purposes.
pub struct InMemoryAgentBackend {
    agents: Arc<Mutex<HashMap<String, AgentInfo>>>,
    models: Arc<Mutex<HashMap<String, ModelInfo>>>,
}

impl InMemoryAgentBackend {
    /// Create a new in-memory agent backend for testing
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryAgentBackend {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl AgentBackend for InMemoryAgentBackend {
    async fn deploy_agent(&self, config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        let agent_info = AgentInfo {
            name: config.name.clone(),
            agent_id: format!("test-agent-{}", config.name),
            model: config.model.clone(),
            status: AgentStatus::Running,
            replicas: 1,
            capabilities: config.capabilities.clone(),
            resources: AgentResourceUsage {
                cpu_millicores: 1000,
                memory_bytes: 1_073_741_824,
                gpu_percent: None,
                network_bytes_per_sec: 0,
            },
            created_at: SystemTime::now(),
            last_updated: SystemTime::now(),
        };

        self.agents
            .lock()
            .await
            .insert(config.name.clone(), agent_info.clone());

        tracing::debug!("Deployed test agent: {}", config.name);
        Ok(agent_info)
    }

    async fn load_model(&self, config: &ModelConfig) -> ToadStoolResult<ModelInfo> {
        let model_info = ModelInfo {
            name: config.name.clone(),
            model_id: format!("test-model-{}", config.name),
            model_type: config.model_type.clone(),
            size_bytes: 1_000_000_000,
            status: ModelStatus::Ready,
            resource_requirements: ModelResourceRequirements {
                min_cpu_cores: 2.0,
                min_memory_gb: 4.0,
                gpu_required: false,
                min_gpu_memory_gb: None,
            },
            performance: ModelPerformanceMetrics {
                avg_inference_time_ms: 100,
                throughput_rps: 10.0,
                success_rate: 99.9,
            },
            loaded_at: SystemTime::now(),
        };

        self.models
            .lock()
            .await
            .insert(config.name.clone(), model_info.clone());

        tracing::debug!("Loaded test model: {}", config.name);
        Ok(model_info)
    }

    async fn scale_agent(&self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(agent_name) {
            agent.replicas = replicas;
            agent.last_updated = SystemTime::now();
            tracing::debug!("Scaled test agent {} to {} replicas", agent_name, replicas);
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Agent {agent_name} not found"
            )))
        }
    }

    async fn stop_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(agent_name) {
            agent.status = AgentStatus::Stopped;
            agent.last_updated = SystemTime::now();
            tracing::debug!("Stopped test agent {}", agent_name);
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Agent {agent_name} not found"
            )))
        }
    }

    async fn remove_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        self.agents
            .lock()
            .await
            .remove(agent_name)
            .ok_or_else(|| ToadStoolError::not_found(format!("Agent {agent_name} not found")))?;

        tracing::debug!("Removed test agent {}", agent_name);
        Ok(())
    }

    async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus> {
        let agents = self.agents.lock().await;
        agents
            .get(agent_name)
            .map(|agent| agent.status.clone())
            .ok_or_else(|| ToadStoolError::not_found(format!("Agent {agent_name} not found")))
    }

    async fn list_agents(&self) -> ToadStoolResult<Vec<AgentInfo>> {
        let agents = self.agents.lock().await;
        Ok(agents.values().cloned().collect())
    }

    async fn list_models(&self) -> ToadStoolResult<Vec<ModelInfo>> {
        let models = self.models.lock().await;
        Ok(models.values().cloned().collect())
    }

    async fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage> {
        let agents = self.agents.lock().await;
        agents
            .get(agent_name)
            .map(|agent| agent.resources.clone())
            .ok_or_else(|| ToadStoolError::not_found(format!("Agent {agent_name} not found")))
    }

    async fn unload_model(&self, model_name: &str) -> ToadStoolResult<()> {
        self.models
            .lock()
            .await
            .remove(model_name)
            .ok_or_else(|| ToadStoolError::not_found(format!("Model {model_name} not found")))?;

        tracing::debug!("Unloaded test model {}", model_name);
        Ok(())
    }
}
