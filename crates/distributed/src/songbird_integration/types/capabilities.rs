//! Capability announcement and tracking types

use std::collections::HashMap;

use chrono::{DateTime, Utc};

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
    pub timestamp: DateTime<Utc>,
    pub capabilities: HashMap<NodeId, NodeCapabilities>,
}
