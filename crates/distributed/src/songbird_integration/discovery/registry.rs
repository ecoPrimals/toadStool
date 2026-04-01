// SPDX-License-Identifier: AGPL-3.0-only
//! Node registry, capability tracker, and health monitor implementations

use std::collections::HashMap;
use std::time::Duration;

use toadstool::error::ToadStoolResult;

use super::super::types::{
    CapabilityTracker, NetworkHealthMonitor, NodeId, NodeRegistration, NodeRegistry, NodeType,
};

impl NodeRegistry {
    /// Create an empty node registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return references to all nodes currently considered active.
    pub fn get_active_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    /// Return references to every registered node.
    pub fn get_all_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    /// Filter nodes whose type matches one of the given canonical node types.
    pub fn get_nodes_by_types(&self, types: &[NodeType]) -> Vec<&NodeRegistration> {
        self.nodes
            .values()
            .filter(|node| {
                types.iter().any(|t| {
                    matches!(
                        (t, &node.node_type),
                        (NodeType::ToadStool, NodeType::ToadStool)
                            | (NodeType::NestGate, NodeType::NestGate)
                            | (NodeType::BearDog, NodeType::BearDog)
                            | (NodeType::Songbird, NodeType::Songbird)
                    )
                })
            })
            .collect()
    }

    /// Insert or update a node registration in the registry.
    pub fn register_node(&mut self, registration: NodeRegistration) -> ToadStoolResult<()> {
        self.register(registration);
        Ok(())
    }

    /// No-op placeholder when discovery reports health for a known node id.
    pub fn update_node_health(&mut self, node_id: &NodeId, _healthy: bool) {
        // Mark node as active if it exists
        if self.nodes.contains_key(node_id) {
            // Node remains in registry as active
        }
    }
}

impl NetworkHealthMonitor {
    /// Create a monitor with the given check interval (stored as `check_interval`).
    pub fn new(timeout: Duration) -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: timeout,
        }
    }
}

impl Clone for NetworkHealthMonitor {
    fn clone(&self) -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: self.last_check,
            check_interval: self.check_interval,
        }
    }
}

impl CapabilityTracker {
    /// Create an empty capability tracker.
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }
}

impl Clone for CapabilityTracker {
    fn clone(&self) -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }
}
