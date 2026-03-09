// SPDX-License-Identifier: AGPL-3.0-only
//! Types and enums for resource estimation
//!
//! Re-exported from parent module for backward compatibility.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::graph_types::GraphValidationError;

/// Resource estimate for an execution graph
///
/// Provides complete resource profile including CPU, memory, GPU, storage,
/// network bandwidth, and estimated execution duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEstimate {
    /// Graph ID this estimate is for
    pub graph_id: String,

    /// Total CPU cores needed (peak)
    pub cpu_cores: u32,

    /// Total memory needed in bytes (peak)
    pub memory_bytes: u64,

    /// Total GPU memory needed in bytes (peak)
    pub gpu_memory_bytes: u64,

    /// Total storage needed in bytes
    pub storage_bytes: u64,

    /// Network bandwidth needed in Mbps
    pub network_bandwidth_mbps: u64,

    /// Estimated execution duration
    pub estimated_duration: Duration,

    /// Maximum parallelism level (number of concurrent nodes)
    pub max_parallelism: usize,

    /// Critical path length (longest dependency chain)
    pub critical_path_length: usize,

    /// Per-node estimates
    pub node_estimates: std::collections::HashMap<String, NodeEstimate>,

    /// Optional warnings or notes
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Resource estimate for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEstimate {
    /// Node ID
    pub node_id: String,

    /// CPU cores needed
    pub cpu_cores: u32,

    /// Memory needed in bytes
    pub memory_bytes: u64,

    /// GPU memory needed in bytes
    pub gpu_memory_bytes: u64,

    /// Estimated duration
    pub duration: Duration,

    /// Parallelism level (which parallel group this node belongs to)
    pub parallelism_level: usize,
}

/// Estimation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum EstimationError {
    #[error("Invalid graph: {0}")]
    InvalidGraph(#[from] GraphValidationError),

    #[error("Graph contains cycles (not a DAG)")]
    CyclicGraph,

    #[error("Unable to estimate node '{0}': {1}")]
    NodeEstimationFailed(String, String),
}
