//! Shared node and capability types (no circular deps)

use serde::{Deserialize, Serialize};

/// Network node identifier
pub type NodeId = String;

/// Node capability specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: u32,
    pub specialized_hardware: Vec<String>,
    pub software_capabilities: Vec<String>,
}
