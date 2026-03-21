// SPDX-License-Identifier: AGPL-3.0-only
//! Capability announcement and tracking types

use std::collections::HashMap;

use std::time::SystemTime;

use super::node::{NodeCapabilities, NodeId};

#[derive(Default)]
pub struct CapabilityTracker {
    pub capabilities: HashMap<NodeId, NodeCapabilities>,
}

impl CapabilityTracker {
    pub fn update_capabilities(&mut self, node_id: NodeId, capabilities: NodeCapabilities) {
        self.capabilities.insert(node_id, capabilities);
    }

    pub fn get_capabilities(&self, node_id: &NodeId) -> Option<&NodeCapabilities> {
        self.capabilities.get(node_id)
    }
}

/// Snapshot of node capabilities at a point in time
pub struct CapabilitySnapshot {
    pub timestamp: SystemTime,
    pub capabilities: HashMap<NodeId, NodeCapabilities>,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_capabilities(cpu: f64, memory_gb: f64) -> NodeCapabilities {
        NodeCapabilities {
            cpu_cores: cpu,
            memory_gb,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        }
    }

    #[test]
    fn test_capability_tracker_update_and_get() {
        let mut tracker = CapabilityTracker::default();
        tracker.update_capabilities("node-1".to_string(), make_capabilities(4.0, 8.0));
        let caps = tracker.get_capabilities(&"node-1".to_string()).unwrap();
        assert!((caps.cpu_cores - 4.0).abs() < f64::EPSILON);
        assert!((caps.memory_gb - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_capability_tracker_missing_returns_none() {
        let tracker = CapabilityTracker::default();
        assert!(tracker.get_capabilities(&"missing".to_string()).is_none());
    }

    #[test]
    fn test_capability_tracker_overwrite() {
        let mut tracker = CapabilityTracker::default();
        tracker.update_capabilities("node-1".to_string(), make_capabilities(2.0, 4.0));
        tracker.update_capabilities("node-1".to_string(), make_capabilities(8.0, 16.0));
        let caps = tracker.get_capabilities(&"node-1".to_string()).unwrap();
        assert!((caps.cpu_cores - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_capability_snapshot_construction() {
        let mut caps_map = HashMap::new();
        caps_map.insert("n1".to_string(), make_capabilities(4.0, 8.0));
        let snapshot = CapabilitySnapshot {
            timestamp: SystemTime::now(),
            capabilities: caps_map,
        };
        assert_eq!(snapshot.capabilities.len(), 1);
    }
}
