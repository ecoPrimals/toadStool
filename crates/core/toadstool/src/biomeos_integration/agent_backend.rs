//! Agent backend traits and implementations for BiomeOS/Squirrel integration
//!
//! This module defines the trait interface for agent deployment backends and
//! provides production and test implementations using proper dependency injection.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

use super::types::{AgentConfig, ModelConfig};
use crate::{ToadStoolError, ToadStoolResult};
#[allow(deprecated)]
use toadstool_config::constants::primals::SQUIRREL;

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInfo {
    /// Agent name
    pub name: String,
    /// Agent ID in Squirrel
    pub agent_id: String,
    /// Model being used
    pub model: String,
    /// Agent status
    pub status: AgentStatus,
    /// Replica count
    pub replicas: u32,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Resource usage
    pub resources: AgentResourceUsage,
    /// Creation time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Last update time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_updated: SystemTime,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model ID in Squirrel
    pub model_id: String,
    /// Model type
    pub model_type: String,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Model status
    pub status: ModelStatus,
    /// Resource requirements
    pub resource_requirements: ModelResourceRequirements,
    /// Performance metrics
    pub performance: ModelPerformanceMetrics,
    /// Load time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub loaded_at: SystemTime,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is being deployed
    Deploying,
    /// Agent is running and ready
    Running,
    /// Agent is scaling
    Scaling,
    /// Agent is being updated
    Updating,
    /// Agent is being terminated
    Terminating,
    /// Agent has failed
    Failed(String),
    /// Agent is stopped
    Stopped,
}

/// Model status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    /// Model is being loaded
    Loading,
    /// Model is loaded and ready
    Ready,
    /// Model is being updated
    Updating,
    /// Model is being unloaded
    Unloading,
    /// Model load failed
    Error(String),
}

/// Agent resource usage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentResourceUsage {
    /// CPU usage in millicores
    pub cpu_millicores: u64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// GPU usage percentage
    pub gpu_percent: Option<f32>,
    /// Network bandwidth in bytes/sec
    pub network_bytes_per_sec: u64,
}

/// Model resource requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelResourceRequirements {
    /// Minimum CPU cores
    pub min_cpu_cores: f32,
    /// Minimum memory in GB
    pub min_memory_gb: f32,
    /// GPU required
    pub gpu_required: bool,
    /// Minimum GPU memory in GB
    pub min_gpu_memory_gb: Option<f32>,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPerformanceMetrics {
    /// Average inference time in ms
    pub avg_inference_time_ms: u64,
    /// Throughput in requests/sec
    pub throughput_rps: f32,
    /// Success rate percentage
    pub success_rate: f32,
}

/// Trait defining the interface for agent deployment backends
///
/// This allows dependency injection of different agent deployment implementations
/// (production Squirrel backend, in-memory test backend, etc.) without relying
/// on feature flags or conditional compilation.
#[async_trait]
pub trait AgentBackend: Send + Sync {
    /// Initialize/test connection to agent backend
    ///
    /// For network backends (Squirrel), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    async fn initialize(&self) -> ToadStoolResult<()> {
        Ok(()) // Default implementation is no-op
    }

    /// Deploy an AI agent from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Agent configuration is invalid
    /// - Backend service is unavailable
    /// - Resource allocation fails
    /// - Agent name conflicts with existing agent
    async fn deploy_agent(&self, config: &AgentConfig) -> ToadStoolResult<AgentInfo>;

    /// Load a model for agent use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Model configuration is invalid
    /// - Model file cannot be accessed or downloaded
    /// - Insufficient memory for model
    /// - Model format is unsupported
    async fn load_model(&self, config: &ModelConfig) -> ToadStoolResult<ModelInfo>;

    /// Scale an agent to specified replica count
    async fn scale_agent(&self, agent_name: &str, replicas: u32) -> ToadStoolResult<()>;

    /// Stop an agent
    async fn stop_agent(&self, agent_name: &str) -> ToadStoolResult<()>;

    /// Remove an agent
    async fn remove_agent(&self, agent_name: &str) -> ToadStoolResult<()>;

    /// Get agent status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Agent does not exist
    /// - Backend service is unavailable
    /// - Network communication fails
    async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus>;

    /// List all deployed agents
    async fn list_agents(&self) -> ToadStoolResult<Vec<AgentInfo>>;

    /// List all loaded models
    async fn list_models(&self) -> ToadStoolResult<Vec<ModelInfo>>;

    /// Get agent resource usage
    async fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage>;

    /// Unload a model
    async fn unload_model(&self, model_name: &str) -> ToadStoolResult<()>;

    /// Health check for agent backend
    async fn health_check(&self) -> ToadStoolResult<()> {
        Ok(()) // Default implementation
    }
}

/// Production implementation using Squirrel Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct SquirrelBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    #[allow(dead_code)] // Stored for potential future use
    model_registry: String,
    #[allow(dead_code)] // Stored for potential future use
    agent_runtime: String,
    _mcp_enabled: bool,
}

impl SquirrelBackend {
    /// Create ML agent backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers ML/MCP service by capability, not name.
    /// Works with ANY service providing ml.agent capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    pub async fn new_async(
        model_registry: impl Into<String>,
        agent_runtime: impl Into<String>,
        mcp_enabled: bool,
    ) -> ToadStoolResult<Self> {
        use toadstool_common::primal_identity::Capability;

        // CAPABILITY-BASED: Discover ANY ML service (not hardcoded "squirrel")
        let socket_path =
            toadstool_common::primal_sockets::discover_socket_for_capability(Capability::Custom {
                name: "ml.agent".to_string(),
                version: "1.0".to_string(),
            })
            .await
            .or_else(|_| {
                // Fallback: Try MCP capability
                futures::executor::block_on(
                    toadstool_common::primal_sockets::discover_socket_for_capability(
                        Capability::Custom {
                            name: "mcp".to_string(),
                            version: "1.0".to_string(),
                        },
                    ),
                )
            })
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "No ML/MCP service discovered: {e}. Ensure a ML provider is running."
                ))
            })?;

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            model_registry: model_registry.into(),
            agent_runtime: agent_runtime.into(),
            _mcp_enabled: mcp_enabled,
        })
    }

    /// Create a new ML agent backend with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    #[must_use]
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    #[allow(deprecated)]
    pub fn new(
        _endpoint: impl Into<String>,
        model_registry: impl Into<String>,
        agent_runtime: impl Into<String>,
        mcp_enabled: bool,
    ) -> Self {
        // LEGACY: Uses primal name for backward compatibility
        let socket_path = toadstool_common::primal_sockets::get_socket_path_for_service(SQUIRREL);
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            model_registry: model_registry.into(),
            agent_runtime: agent_runtime.into(),
            _mcp_enabled: mcp_enabled,
        }
    }
}

#[async_trait]
impl AgentBackend for SquirrelBackend {
    async fn initialize(&self) -> ToadStoolResult<()> {
        // Health check via JSON-RPC over unix socket
        let _health: serde_json::Value = self
            .rpc_client
            .call("squirrel.health", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to connect to Squirrel: {e}")))?;

        tracing::info!("Successfully connected to Squirrel via unix socket");
        Ok(())
    }

    async fn deploy_agent(&self, config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        let params = serde_json::to_value(config)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize config: {e}")))?;

        let agent_info: AgentInfo = self
            .rpc_client
            .call_typed("squirrel.deploy_agent", params)
            .await
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to deploy agent {}: {}", config.name, e))
            })?;

        tracing::info!("Deployed agent: {}", config.name);
        Ok(agent_info)
    }

    async fn load_model(&self, config: &ModelConfig) -> ToadStoolResult<ModelInfo> {
        let params = serde_json::to_value(config)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize config: {e}")))?;

        let model_info: ModelInfo = self
            .rpc_client
            .call_typed("squirrel.load_model", params)
            .await
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to load model {}: {}", config.name, e))
            })?;

        tracing::info!("Loaded model: {}", config.name);
        Ok(model_info)
    }

    async fn scale_agent(&self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        let params = serde_json::json!({
            "agent_name": agent_name,
            "replicas": replicas
        });

        let _: serde_json::Value = self
            .rpc_client
            .call("squirrel.scale_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to scale agent: {e}")))?;

        tracing::info!("Scaled agent {} to {} replicas", agent_name, replicas);
        Ok(())
    }

    async fn stop_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"agent_name": agent_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("squirrel.stop_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to stop agent: {e}")))?;

        tracing::info!("Stopped agent {}", agent_name);
        Ok(())
    }

    async fn remove_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"agent_name": agent_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("squirrel.remove_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to remove agent: {e}")))?;

        tracing::info!("Removed agent {}", agent_name);
        Ok(())
    }

    async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus> {
        let params = serde_json::json!({"agent_name": agent_name});

        let status: AgentStatus = self
            .rpc_client
            .call_typed("squirrel.get_agent_status", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to get agent status: {e}")))?;

        Ok(status)
    }

    async fn list_agents(&self) -> ToadStoolResult<Vec<AgentInfo>> {
        let agents: Vec<AgentInfo> = self
            .rpc_client
            .call_typed("squirrel.list_agents", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to list agents: {e}")))?;

        Ok(agents)
    }

    async fn list_models(&self) -> ToadStoolResult<Vec<ModelInfo>> {
        let models: Vec<ModelInfo> = self
            .rpc_client
            .call_typed("squirrel.list_models", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to list models: {e}")))?;

        Ok(models)
    }

    async fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage> {
        let params = serde_json::json!({"agent_name": agent_name});

        let resources: AgentResourceUsage = self
            .rpc_client
            .call_typed("squirrel.get_agent_resources", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to get agent resources: {e}")))?;

        Ok(resources)
    }

    async fn unload_model(&self, model_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"model_name": model_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("squirrel.unload_model", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to unload model: {e}")))?;

        tracing::info!("Unloaded model {}", model_name);
        Ok(())
    }
}

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

        let mut agents = self.agents.lock().await;
        agents.insert(config.name.clone(), agent_info.clone());

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

        let mut models = self.models.lock().await;
        models.insert(config.name.clone(), model_info.clone());

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
        let mut agents = self.agents.lock().await;
        agents
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
        let mut models = self.models.lock().await;
        models
            .remove(model_name)
            .ok_or_else(|| ToadStoolError::not_found(format!("Model {model_name} not found")))?;

        tracing::debug!("Unloaded test model {}", model_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_agent_backend_deploy() {
        let backend = InMemoryAgentBackend::new();
        let config = AgentConfig {
            name: "test-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };

        let result = backend.deploy_agent(&config).await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "test-agent");
        assert_eq!(info.status, AgentStatus::Running);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_agent_backend_lifecycle() {
        let backend = InMemoryAgentBackend::new();
        let config = AgentConfig {
            name: "lifecycle-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };

        // Deploy
        backend.deploy_agent(&config).await.unwrap();

        // Scale
        backend.scale_agent("lifecycle-agent", 3).await.unwrap();

        // Check status
        let status = backend.get_agent_status("lifecycle-agent").await.unwrap();
        assert_eq!(status, AgentStatus::Running);

        // Stop
        backend.stop_agent("lifecycle-agent").await.unwrap();
        let status = backend.get_agent_status("lifecycle-agent").await.unwrap();
        assert_eq!(status, AgentStatus::Stopped);

        // Remove
        backend.remove_agent("lifecycle-agent").await.unwrap();

        // Verify removed
        let status_result = backend.get_agent_status("lifecycle-agent").await;
        assert!(status_result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_agent_backend_list() {
        let backend = InMemoryAgentBackend::new();

        // Initially empty
        let list = backend.list_agents().await.unwrap();
        assert_eq!(list.len(), 0);

        // Deploy agents
        for i in 1..=3 {
            let config = AgentConfig {
                name: format!("agent-{}", i),
                model: "test-model".to_string(),
                capabilities: vec!["chat".to_string()],
                resources: None,
                environment: HashMap::new(),
                config: HashMap::new(),
            };
            backend.deploy_agent(&config).await.unwrap();
        }

        // List should have 3 agents
        let list = backend.list_agents().await.unwrap();
        assert_eq!(list.len(), 3);
    }
}
