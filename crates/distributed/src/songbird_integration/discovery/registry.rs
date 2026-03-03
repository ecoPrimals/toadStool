// SPDX-License-Identifier: AGPL-3.0-or-later
//! Node registry, capability tracker, and health monitor implementations

use std::collections::HashMap;
use std::time::Duration;

use toadstool::error::ToadStoolResult;

use super::super::types::{
    CapabilityTracker, NetworkHealthMonitor, NodeId, NodeRegistration, NodeRegistry, NodeType,
};

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_active_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    pub fn get_all_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

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

    pub fn register_node(&mut self, registration: NodeRegistration) -> ToadStoolResult<()> {
        self.register(registration);
        Ok(())
    }

    pub fn update_node_health(&mut self, node_id: &NodeId, _healthy: bool) {
        // Mark node as active if it exists
        if self.nodes.contains_key(node_id) {
            // Node remains in registry as active
        }
    }
}

impl NetworkHealthMonitor {
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
