// SPDX-License-Identifier: AGPL-3.0-or-later
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
                            | (NodeType::Storage, NodeType::Storage)
                            | (NodeType::Security, NodeType::Security)
                            | (NodeType::Coordination, NodeType::Coordination)
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
                self.health_timestamps
                    .insert(node_id.clone(), Instant::now());
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
                    .is_some_and(|ts| now.duration_since(*ts) <= max_age)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::types::{NodeCapabilities, NodeMetadata, NodeRegistration, NodeType};

    fn sample_registration(node_id: &str, node_type: NodeType) -> NodeRegistration {
        let caps = NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        };
        NodeRegistration {
            node_id: node_id.to_string(),
            node_type,
            capabilities: caps.clone(),
            endpoints: vec!["http://127.0.0.1:1".to_string()],
            protocols: vec!["http".to_string()],
            metadata: NodeMetadata {
                version: "1".to_string(),
                build_info: "test".to_string(),
                capabilities: caps,
            },
        }
    }

    #[test]
    fn node_registry_new_matches_default() {
        let a = NodeRegistry::new();
        let b = NodeRegistry::default();
        assert_eq!(a.nodes.len(), b.nodes.len());
    }

    #[test]
    fn register_node_inserts_and_lists_node() {
        let mut reg = NodeRegistry::new();
        let node = sample_registration("n1", NodeType::ToadStool);
        reg.register_node(node.clone()).unwrap();
        assert_eq!(reg.get_all_nodes().len(), 1);
        assert_eq!(reg.get_active_nodes().len(), 1);
        assert!(reg.get_node(&"n1".to_string()).is_some());
    }

    #[test]
    fn get_nodes_by_types_filters_matching_kinds_only() {
        let mut reg = NodeRegistry::new();
        reg.register_node(sample_registration("t", NodeType::ToadStool));
        reg.register_node(sample_registration("s", NodeType::Storage));
        let ts = reg.get_nodes_by_types(&[NodeType::ToadStool]);
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].node_id, "t");
    }

    #[test]
    fn update_node_health_false_removes_node() {
        let mut reg = NodeRegistry::new();
        reg.register_node(sample_registration("gone", NodeType::ToadStool));
        reg.update_node_health(&"gone".to_string(), false);
        assert!(reg.get_node(&"gone".to_string()).is_none());
        assert!(reg.health_timestamps.get("gone").is_none());
    }

    #[test]
    fn update_node_health_true_unknown_id_is_ignored() {
        let mut reg = NodeRegistry::new();
        reg.update_node_health(&"missing".to_string(), true);
        assert!(reg.nodes.is_empty());
    }

    #[test]
    fn get_healthy_nodes_respects_max_age() {
        let mut reg = NodeRegistry::new();
        reg.register_node(sample_registration("fresh", NodeType::ToadStool));
        assert_eq!(reg.get_healthy_nodes(Duration::from_secs(3600)).len(), 1);

        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(reg.get_healthy_nodes(Duration::from_secs(3600)).len(), 1);
        assert!(reg.get_healthy_nodes(Duration::from_millis(50)).is_empty());
    }

    #[test]
    fn network_health_monitor_new_sets_check_interval() {
        let interval = Duration::from_secs(42);
        let m = NetworkHealthMonitor::new(interval);
        assert_eq!(m.check_interval, interval);
        assert!(m.health_checks.is_empty());
    }

    #[test]
    fn network_health_monitor_clone_clears_ephemeral_state() {
        let mut m = NetworkHealthMonitor::new(Duration::from_secs(1));
        m.last_check = Some(std::time::SystemTime::UNIX_EPOCH);
        let c = m.clone();
        assert!(c.health_checks.is_empty());
        assert_eq!(c.check_interval, m.check_interval);
    }

    #[test]
    fn capability_tracker_new_is_empty() {
        let t = CapabilityTracker::new();
        assert!(t.capabilities.is_empty());
    }

    #[test]
    fn capability_tracker_clone_clears_map() {
        let mut t = CapabilityTracker::new();
        let caps = NodeCapabilities {
            cpu_cores: 1.0,
            memory_gb: 1.0,
            storage_gb: 1.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        };
        t.capabilities.insert("k".to_string(), caps);
        let c = t.clone();
        assert!(c.capabilities.is_empty());
    }
}
