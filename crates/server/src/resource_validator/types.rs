// SPDX-License-Identifier: AGPL-3.0-only
//! Public types for resource validation results and system capability snapshots.

use serde::{Deserialize, Serialize};

use crate::resource_estimator::ResourceEstimate;

/// Resource availability result
///
/// Reports whether the system can execute the graph and identifies any
/// resource gaps that would prevent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    /// Graph ID this result is for
    pub graph_id: String,

    /// Whether the system can execute this graph
    pub available: bool,

    /// Resource gaps (what's missing)
    pub gaps: Vec<ResourceGap>,

    /// Warnings (resources are tight but available)
    pub warnings: Vec<String>,

    /// System capabilities at time of check
    pub system_capabilities: SystemCapabilities,

    /// Estimated requirements
    pub estimated_requirements: ResourceEstimate,
}

/// A resource gap preventing execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGap {
    /// Resource type (cpu, memory, gpu, etc.)
    pub resource_type: String,

    /// Required amount
    pub required: u64,

    /// Available amount
    pub available: u64,

    /// Shortage amount
    pub shortage: u64,

    /// Suggested action
    pub suggestion: String,
}

/// System capabilities snapshot
///
/// Represents the system's current resource availability.
/// All values are discovered at runtime, no hardcoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// Total CPU cores
    pub total_cpu_cores: u32,

    /// Available CPU cores (not in use)
    pub available_cpu_cores: u32,

    /// Total memory in bytes
    pub total_memory_bytes: u64,

    /// Available memory in bytes
    pub available_memory_bytes: u64,

    /// Total GPU memory in bytes (across all GPUs)
    pub total_gpu_memory_bytes: u64,

    /// Available GPU memory in bytes
    pub available_gpu_memory_bytes: u64,

    /// Total storage in bytes
    pub total_storage_bytes: u64,

    /// Available storage in bytes
    pub available_storage_bytes: u64,

    /// Network bandwidth in Mbps
    pub network_bandwidth_mbps: u64,

    /// GPU count
    pub gpu_count: usize,

    /// GPU types (e.g., "NVIDIA RTX 3090", "AMD RX 6950 XT")
    pub gpu_types: Vec<String>,
}
