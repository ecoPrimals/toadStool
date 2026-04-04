// SPDX-License-Identifier: AGPL-3.0-only
//! Resource requirement types (CPU, memory, storage, network, GPU).

use serde::{Deserialize, Serialize};

use crate::ToadStoolResult;

/// Resource requirements specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// GPU requirements (optional)
    pub gpu: Option<GpuRequirements>,
    /// Network requirements
    pub network: NetworkRequirements,
}

/// CPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores
    pub min_cores: f64,
    /// Maximum CPU cores
    pub max_cores: Option<f64>,
    /// CPU architecture requirement
    pub architecture: Option<String>,
}

impl Default for CpuRequirements {
    fn default() -> Self {
        Self {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        }
    }
}

/// Memory requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes
    pub min_bytes: u64,
    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
}

impl Default for MemoryRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
        }
    }
}

/// Storage requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes
    pub min_bytes: u64,
    /// Maximum storage in bytes
    pub max_bytes: Option<u64>,
    /// Storage type requirement
    pub storage_type: Option<String>,
}

impl Default for StorageRequirements {
    fn default() -> Self {
        Self {
            min_bytes: 1024 * 1024 * 1024, // 1GB
            max_bytes: None,
            storage_type: None,
        }
    }
}

/// Network requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in bytes per second
    pub min_bandwidth: Option<u64>,
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth: Option<u64>,
    /// Latency requirement in milliseconds
    pub max_latency_ms: Option<u64>,
}

impl ResourceRequirements {
    /// Validate that the requirements are internally consistent.
    ///
    /// # Errors
    ///
    /// Returns error if CPU or memory bounds are invalid.
    pub fn validate(&self) -> ToadStoolResult<()> {
        use crate::ToadStoolError;
        if self.cpu.min_cores <= 0.0 {
            return Err(ToadStoolError::validation(
                "cpu.min_cores must be greater than 0",
            ));
        }
        if self.memory.min_bytes == 0 {
            return Err(ToadStoolError::validation(
                "memory.min_bytes must be greater than 0",
            ));
        }
        Ok(())
    }
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU units
    pub min_units: u32,
    /// Maximum GPU units
    pub max_units: Option<u32>,
    /// GPU type requirement
    pub gpu_type: Option<String>,
    /// Minimum GPU memory in bytes
    pub min_memory_bytes: Option<u64>,
}
