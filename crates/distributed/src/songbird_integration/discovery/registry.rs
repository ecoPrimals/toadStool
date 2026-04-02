// SPDX-License-Identifier: AGPL-3.0-only
//! Node registry, capability tracker, and health monitor implementations

use std::collections::HashMap;
use std::time::{Duration, Instant};

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

    /// Update the health state for a known node.
    ///
    /// Healthy nodes are kept in the registry. Unhealthy nodes are removed so
    /// they stop appearing in `get_active_nodes` results. Unknown nodes are
    /// silently ignored (not yet registered).
    pub fn update_node_health(&mut self, node_id: &NodeId, healthy: bool) {
        if healthy {
            if let Some(entry) = self.health_timestamps.get_mut(node_id) {
                *entry = Instant::now();
            } else if self.nodes.contains_key(node_id) {
                self.health_timestamps.insert(node_id.clone(), Instant::now());
            }
        } else {
            self.nodes.remove(node_id);
            self.health_timestamps.remove(node_id);
            tracing::info!("Removed unhealthy node {} from registry", node_id);
        }
    }

    /// Return references to nodes whose health heartbeat is within `max_age`.
    pub fn get_healthy_nodes(&self, max_age: Duration) -> Vec<&NodeRegistration> {
        let now = Instant::now();
        self.nodes
            .iter()
            .filter(|(id, _)| {
                self.health_timestamps
                    .get(*id)
                    .map_or(false, |ts| now.duration_since(*ts) <= max_age)
            })
            .map(|(_, reg)| reg)
            .collect()
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
