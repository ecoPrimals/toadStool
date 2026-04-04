// SPDX-License-Identifier: AGPL-3.0-only
use async_trait::async_trait;

use super::super::types::{AgentConfig, ModelConfig};
use super::AgentBackend;
use super::types::{AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo};
use crate::{ToadStoolError, ToadStoolResult};

/// Production implementation using Squirrel Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct SquirrelBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    _model_registry: String,
    _agent_runtime: String,
    _mcp_enabled: bool,
}

impl SquirrelBackend {
    /// Create ML agent backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers ML/MCP service by capability, not name.
    /// Works with ANY service providing ml.agent capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or the ML service socket cannot be connected.
    pub async fn new_async(
        model_registry: impl Into<String>,
        agent_runtime: impl Into<String>,
        mcp_enabled: bool,
    ) -> ToadStoolResult<Self> {
        use toadstool_common::primal_identity::Capability;

        let socket_path = match toadstool_common::primal_sockets::discover_socket_for_capability(
            Capability::Custom {
                name: "ml.agent".to_string(),
                version: "1.0".to_string(),
            },
        )
        .await
        {
            Ok(path) => path,
            Err(_) => toadstool_common::primal_sockets::discover_socket_for_capability(
                Capability::Custom {
                    name: "mcp".to_string(),
                    version: "1.0".to_string(),
                },
            )
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "No ML/MCP service discovered: {e}. Ensure a ML provider is running."
                ))
            })?,
        };

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _model_registry: model_registry.into(),
            _agent_runtime: agent_runtime.into(),
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
    pub fn new(
        _endpoint: impl Into<String>,
        model_registry: impl Into<String>,
        agent_runtime: impl Into<String>,
        mcp_enabled: bool,
    ) -> Self {
        let socket_path = toadstool_common::primal_sockets::get_socket_path_for_capability("ai");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _model_registry: model_registry.into(),
            _agent_runtime: agent_runtime.into(),
            _mcp_enabled: mcp_enabled,
        }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl AgentBackend for SquirrelBackend {
    async fn initialize(&self) -> ToadStoolResult<()> {
        // Health check via JSON-RPC over unix socket
        let _health: serde_json::Value = self
            .rpc_client
            .call("ai.health", serde_json::json!({}))
            .await
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to connect to AI/routing service: {e}"))
            })?;

        tracing::info!("Successfully connected to AI/routing service via unix socket");
        Ok(())
    }

    async fn deploy_agent(&self, config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        let params = serde_json::to_value(config)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize config: {e}")))?;

        let agent_info: AgentInfo = self
            .rpc_client
            .call_typed("ai.deploy_agent", params)
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
            .call_typed("ai.load_model", params)
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
            .call("ai.scale_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to scale agent: {e}")))?;

        tracing::info!("Scaled agent {} to {} replicas", agent_name, replicas);
        Ok(())
    }

    async fn stop_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"agent_name": agent_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("ai.stop_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to stop agent: {e}")))?;

        tracing::info!("Stopped agent {}", agent_name);
        Ok(())
    }

    async fn remove_agent(&self, agent_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"agent_name": agent_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("ai.remove_agent", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to remove agent: {e}")))?;

        tracing::info!("Removed agent {}", agent_name);
        Ok(())
    }

    async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus> {
        let params = serde_json::json!({"agent_name": agent_name});

        let status: AgentStatus = self
            .rpc_client
            .call_typed("ai.get_agent_status", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to get agent status: {e}")))?;

        Ok(status)
    }

    async fn list_agents(&self) -> ToadStoolResult<Vec<AgentInfo>> {
        let agents: Vec<AgentInfo> = self
            .rpc_client
            .call_typed("ai.list_agents", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to list agents: {e}")))?;

        Ok(agents)
    }

    async fn list_models(&self) -> ToadStoolResult<Vec<ModelInfo>> {
        let models: Vec<ModelInfo> = self
            .rpc_client
            .call_typed("ai.list_models", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to list models: {e}")))?;

        Ok(models)
    }

    async fn get_agent_resources(&self, agent_name: &str) -> ToadStoolResult<AgentResourceUsage> {
        let params = serde_json::json!({"agent_name": agent_name});

        let resources: AgentResourceUsage = self
            .rpc_client
            .call_typed("ai.get_agent_resources", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to get agent resources: {e}")))?;

        Ok(resources)
    }

    async fn unload_model(&self, model_name: &str) -> ToadStoolResult<()> {
        let params = serde_json::json!({"model_name": model_name});

        let _: serde_json::Value = self
            .rpc_client
            .call("ai.unload_model", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to unload model: {e}")))?;

        tracing::info!("Unloaded model {}", model_name);
        Ok(())
    }
}
