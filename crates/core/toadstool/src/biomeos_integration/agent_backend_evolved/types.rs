// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request/response and status types for AI agent and model operations.

use serde::{Deserialize, Serialize};

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier.
    pub id: String,
    /// Human-readable agent name.
    pub name: String,
    /// Model ID backing this agent.
    pub model: String,
    /// Current lifecycle status.
    pub status: AgentStatus,
    /// Number of active replicas.
    pub replicas: u32,
    /// Capability tags (e.g. inference, embedding).
    pub capabilities: Vec<String>,
}

/// Agent lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent is being deployed.
    Deploying,
    /// Agent is running and serving requests.
    Running,
    /// Agent is scaling replicas up or down.
    Scaling,
    /// Agent is stopped.
    Stopped,
    /// Agent deployment or runtime failed.
    Failed,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Model architecture or type (e.g. transformer).
    pub model_type: String,
    /// Size in bytes on disk or memory.
    pub size_bytes: u64,
    /// Current load status.
    pub status: ModelStatus,
}

/// Model load lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// Model is being loaded into memory.
    Loading,
    /// Model is ready for inference.
    Ready,
    /// Model is being unloaded.
    Unloading,
    /// Model load or inference error.
    Error,
}

/// Agent deployment request
#[derive(Debug, Serialize)]
pub struct DeployAgentRequest {
    /// Agent display name.
    pub name: String,
    /// Model ID to run.
    pub model: String,
    /// Desired replica count.
    pub replicas: u32,
    /// Capability tags for routing.
    pub capabilities: Vec<String>,
}

/// Model load request
#[derive(Debug, Serialize)]
pub struct LoadModelRequest {
    /// Model display name.
    pub name: String,
    /// Model architecture or type.
    pub model_type: String,
    /// Source URI (e.g. S3, local path).
    pub source: String,
}
