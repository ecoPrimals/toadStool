// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::future::Future;
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

impl AgentBackend for InMemoryAgentBackend {
    fn deploy_agent<'a>(
        &'a self,
        config: &'a AgentConfig,
    ) -> impl Future<Output = ToadStoolResult<AgentInfo>> + Send + 'a {
        async move {
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
    }

    fn load_model<'a>(
        &'a self,
        config: &'a ModelConfig,
    ) -> impl Future<Output = ToadStoolResult<ModelInfo>> + Send + 'a {
        async move {
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
    }

    fn scale_agent<'a>(
        &'a self,
        agent_name: &'a str,
        replicas: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
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
    }

    fn stop_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
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
    }

    fn remove_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            self.agents.lock().await.remove(agent_name).ok_or_else(|| {
                ToadStoolError::not_found(format!("Agent {agent_name} not found"))
            })?;

            tracing::debug!("Removed test agent {}", agent_name);
            Ok(())
        }
    }

    fn get_agent_status<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentStatus>> + Send + 'a {
        async move {
            let agents = self.agents.lock().await;
            agents
                .get(agent_name)
                .map(|agent| agent.status.clone())
                .ok_or_else(|| ToadStoolError::not_found(format!("Agent {agent_name} not found")))
        }
    }

    fn list_agents(&self) -> impl Future<Output = ToadStoolResult<Vec<AgentInfo>>> + Send + '_ {
        async move {
            let agents = self.agents.lock().await;
            Ok(agents.values().cloned().collect())
        }
    }

    fn list_models(&self) -> impl Future<Output = ToadStoolResult<Vec<ModelInfo>>> + Send + '_ {
        async move {
            let models = self.models.lock().await;
            Ok(models.values().cloned().collect())
        }
    }

    fn get_agent_resources<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentResourceUsage>> + Send + 'a {
        async move {
            let agents = self.agents.lock().await;
            agents
                .get(agent_name)
                .map(|agent| agent.resources.clone())
                .ok_or_else(|| ToadStoolError::not_found(format!("Agent {agent_name} not found")))
        }
    }

    fn unload_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            self.models.lock().await.remove(model_name).ok_or_else(|| {
                ToadStoolError::not_found(format!("Model {model_name} not found"))
            })?;

            tracing::debug!("Unloaded test model {}", model_name);
            Ok(())
        }
    }
}
