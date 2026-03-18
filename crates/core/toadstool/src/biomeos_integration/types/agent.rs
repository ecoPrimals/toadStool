// SPDX-License-Identifier: AGPL-3.0-or-later
//! AI agent deployment configuration types.

use super::resources::PrimalResources;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// AI agent deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// Model to use
    pub model: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Resource requirements
    pub resources: Option<PrimalResources>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name
    pub name: String,
    /// Model type (e.g., "gpt-4", "claude-3")
    pub model_type: String,
    /// Model parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Resource requirements
    pub resources: Option<PrimalResources>,
}

/// MCP (Model Control Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    /// Enable MCP
    pub enabled: bool,
    /// MCP version
    pub version: String,
    /// Protocol settings
    pub protocol: HashMap<String, serde_json::Value>,
}

/// Boot configuration for biomeOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    /// Boot mode (normal, recovery, safe)
    pub mode: String,
    /// Boot timeout
    pub timeout: Duration,
    /// Boot services
    pub services: Vec<String>,
}

/// Storage configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: String,
    /// Storage capacity
    pub capacity: u64,
    /// Storage path
    pub path: std::path::PathBuf,
}
