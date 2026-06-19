// SPDX-License-Identifier: AGPL-3.0-or-later
use std::future::Future;

use super::super::types::{AgentConfig, ModelConfig};
use super::AgentBackend;
use super::types::{AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo};
use crate::{ToadStoolError, ToadStoolResult};

/// Production implementation using intelligence service Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct IntelligenceBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    _model_registry: String,
    _agent_runtime: String,
    _mcp_enabled: bool,
}

impl IntelligenceBackend {
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
}

impl AgentBackend for IntelligenceBackend {
    fn initialize(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
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
    }

    fn deploy_agent<'a>(
        &'a self,
        config: &'a AgentConfig,
    ) -> impl Future<Output = ToadStoolResult<AgentInfo>> + Send + 'a {
        async move {
            let params = serde_json::to_value(config)
                .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize config: {e}")))?;

            let agent_info: AgentInfo = self
                .rpc_client
                .call_typed("ai.deploy_agent", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to deploy agent {}: {}",
                        config.name, e
                    ))
                })?;

            tracing::info!("Deployed agent: {}", config.name);
            Ok(agent_info)
        }
    }

    fn load_model<'a>(
        &'a self,
        config: &'a ModelConfig,
    ) -> impl Future<Output = ToadStoolResult<ModelInfo>> + Send + 'a {
        async move {
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
    }

    fn scale_agent<'a>(
        &'a self,
        agent_name: &'a str,
        replicas: u32,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
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
    }

    fn stop_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            let params = serde_json::json!({"agent_name": agent_name});

            let _: serde_json::Value = self
                .rpc_client
                .call("ai.stop_agent", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to stop agent: {e}")))?;

            tracing::info!("Stopped agent {}", agent_name);
            Ok(())
        }
    }

    fn remove_agent<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            let params = serde_json::json!({"agent_name": agent_name});

            let _: serde_json::Value = self
                .rpc_client
                .call("ai.remove_agent", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to remove agent: {e}")))?;

            tracing::info!("Removed agent {}", agent_name);
            Ok(())
        }
    }

    fn get_agent_status<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentStatus>> + Send + 'a {
        async move {
            let params = serde_json::json!({"agent_name": agent_name});

            let status: AgentStatus = self
                .rpc_client
                .call_typed("ai.get_agent_status", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to get agent status: {e}")))?;

            Ok(status)
        }
    }

    fn list_agents(&self) -> impl Future<Output = ToadStoolResult<Vec<AgentInfo>>> + Send + '_ {
        async move {
            let agents: Vec<AgentInfo> = self
                .rpc_client
                .call_typed("ai.list_agents", serde_json::json!({}))
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list agents: {e}")))?;

            Ok(agents)
        }
    }

    fn list_models(&self) -> impl Future<Output = ToadStoolResult<Vec<ModelInfo>>> + Send + '_ {
        async move {
            let models: Vec<ModelInfo> = self
                .rpc_client
                .call_typed("ai.list_models", serde_json::json!({}))
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list models: {e}")))?;

            Ok(models)
        }
    }

    fn get_agent_resources<'a>(
        &'a self,
        agent_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<AgentResourceUsage>> + Send + 'a {
        async move {
            let params = serde_json::json!({"agent_name": agent_name});

            let resources: AgentResourceUsage = self
                .rpc_client
                .call_typed("ai.get_agent_resources", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to get agent resources: {e}"))
                })?;

            Ok(resources)
        }
    }

    fn unload_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
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
}
