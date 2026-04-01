// SPDX-License-Identifier: AGPL-3.0-only
//! Shared node and capability types (no circular deps)

use serde::{Deserialize, Serialize};

/// Network node identifier
pub type NodeId = String;

/// Node capability specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Logical CPU cores advertised by the node.
    pub cpu_cores: f64,
    /// RAM in gigabytes.
    pub memory_gb: f64,
    /// Storage in gigabytes.
    pub storage_gb: f64,
    /// Number of GPUs (or accelerator slots) available.
    pub gpu_count: u32,
    /// Hardware labels (e.g. device families).
    pub specialized_hardware: Vec<String>,
    /// Software features (runtimes, frameworks) available on the node.
    pub software_capabilities: Vec<String>,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;

    #[test]
    fn test_node_capabilities_construction() {
        let caps = NodeCapabilities {
            cpu_cores: 8.0,
            memory_gb: 32.0,
            storage_gb: 500.0,
            gpu_count: 2,
            specialized_hardware: vec!["nvidia".to_string()],
            software_capabilities: vec!["cuda".to_string()],
        };
        assert_eq!(caps.cpu_cores, 8.0);
        assert_eq!(caps.gpu_count, 2);
        assert_eq!(caps.specialized_hardware.len(), 1);
    }

    #[test]
    fn test_node_capabilities_serialization_roundtrip() {
        let caps = NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 16.0,
            storage_gb: 256.0,
            gpu_count: 1,
            specialized_hardware: vec![],
            software_capabilities: vec!["wasm".to_string()],
        };
        let json = serde_json::to_string(&caps).unwrap();
        let parsed: NodeCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cpu_cores, caps.cpu_cores);
        assert_eq!(parsed.software_capabilities, caps.software_capabilities);
    }
}
