// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};

/// Resource requirements for job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// Network requirements
    pub network: NetworkRequirements,
    /// GPU requirements
    pub gpu: Option<GpuRequirements>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu: CpuRequirements {
                min_cores: 1.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1GB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1GB
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        }
    }
}

/// CPU requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores required.
    pub min_cores: f64,
    /// Maximum CPU cores (optional cap).
    pub max_cores: Option<f64>,
}

/// Memory requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes.
    pub min_bytes: u64,
    /// Maximum memory in bytes (optional cap).
    pub max_bytes: Option<u64>,
}

/// Storage requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes.
    pub min_bytes: u64,
    /// Maximum storage in bytes (optional cap).
    pub max_bytes: Option<u64>,
}

/// Network requirements specification for distributed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in Mbps.
    pub bandwidth_mbps: Option<u64>,
    /// Maximum acceptable latency in ms.
    pub latency_ms: Option<u64>,
}

/// GPU requirements specification for accelerated workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU memory in GB.
    pub min_memory_gb: f64,
    /// Required compute capability (e.g. CUDA 8.0).
    pub compute_capability: Option<String>,
}
