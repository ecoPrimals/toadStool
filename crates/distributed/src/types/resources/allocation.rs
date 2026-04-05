// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Resource allocation for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceAllocation {
    /// CPU allocation in cores
    pub cpu_cores: f64,
    /// Memory allocation in bytes
    pub memory_bytes: u64,
    /// Storage allocation in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bytes per second
    pub network_bandwidth: u64,
    /// GPU allocation (if available)
    pub gpu_allocation: Option<GpuAllocation>,
    /// Custom resource allocations
    pub custom_resources: HashMap<String, ResourceValue>,
}

impl Eq for ResourceAllocation {}

impl std::hash::Hash for ResourceAllocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cpu_cores.to_bits().hash(state);
        self.memory_bytes.hash(state);
        self.storage_bytes.hash(state);
        self.network_bandwidth.hash(state);
    }
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        const GIB: u64 = 1024 * 1024 * 1024;
        Self {
            cpu_cores: 1.0,
            memory_bytes: GIB,
            storage_bytes: 10 * GIB,
            network_bandwidth: 100,
            gpu_allocation: None,
            custom_resources: HashMap::new(),
        }
    }
}

/// Resource allocation strategies for child instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceAllocationStrategy {
    /// Equal share across children.
    Fair,
    /// Proportional to workload size.
    Proportional,
    /// Priority-based allocation.
    Priority,
    /// Custom strategy (plugin name).
    Custom(String),
}

/// GPU allocation information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuAllocation {
    /// GPU device ID
    pub device_id: u32,
    /// Memory allocation in bytes
    pub memory_bytes: u64,
    /// Compute units allocated
    pub compute_units: u32,
}

/// Typed value for custom resource allocations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceValue {
    /// Integer resource value.
    Integer(i64),
    /// Float resource value.
    Float(f64),
    /// String resource value.
    String(String),
    /// Boolean resource value.
    Boolean(bool),
}
