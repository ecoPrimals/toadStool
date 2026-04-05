// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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
