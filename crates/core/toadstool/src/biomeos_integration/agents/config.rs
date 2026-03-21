// SPDX-License-Identifier: AGPL-3.0-only
//! Configuration for the agent deployment manager.

use serde::{Deserialize, Serialize};

/// Configuration for agent deployment manager
///
/// # Evolution Note (Feb 2026)
///
/// The `squirrel_endpoint` field is deprecated. Use capability-based discovery:
///
/// ```rust,ignore
/// // OLD: Hardcoded endpoint
/// let config = AgentDeploymentConfig {
///     squirrel_endpoint: "http://localhost:8080".into(),
///     ..Default::default()
/// };
///
/// // NEW: Capability-based discovery
/// let manager = AgentDeploymentManager::discover().await?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeploymentConfig {
    /// Squirrel endpoint URL
    ///
    /// **DEPRECATED**: Use `AgentDeploymentManager::discover()` for runtime discovery.
    /// When empty, the manager discovers Squirrel via capability lookup.
    #[serde(default)]
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

impl Default for AgentDeploymentConfig {
    fn default() -> Self {
        Self {
            squirrel_endpoint: String::new(), // Empty = use runtime discovery
            model_registry: "local".to_string(),
            agent_runtime: "container".to_string(),
            mcp_enabled: false,
            resource_limits: serde_json::Map::new(),
        }
    }
}
