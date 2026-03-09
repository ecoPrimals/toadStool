// SPDX-License-Identifier: AGPL-3.0-only
//! Service discovery and endpoint types

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use toadstool_common::constants::timeouts;

use super::node::{NodeCapabilities, NodeId};
use super::{ConnectionHealth, SongbirdConnection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    ToadStool,
    NestGate,
    BearDog,
    Songbird,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub capabilities: NodeCapabilities,
    pub endpoints: Vec<String>,
    pub protocols: Vec<String>,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub version: String,
    pub build_info: String,
    pub capabilities: NodeCapabilities,
}

#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_capacity: NodeCapabilities,
    pub current_utilization: f64,
}

#[derive(Debug, Clone)]
pub struct LoadBalancingAdvice {
    pub recommended_nodes: Vec<NodeId>,
    pub load_distribution: HashMap<NodeId, f64>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    pub bandwidth_mbps: Option<u64>,
    pub latency_ms: Option<u64>,
    pub reliability_percent: Option<f64>,
}

pub struct NetworkCapacity {
    pub total_nodes: usize,
    pub total_cpu_cores: f64,
    pub total_memory_gb: f64,
    pub total_storage_gb: f64,
}

pub struct AvailableCapacity {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bandwidth: u64,
}

impl AvailableCapacity {
    pub fn can_handle_job(&self, job: &crate::UniversalJob) -> bool {
        let requirements = &job.resource_requirements;
        if requirements.cpu.min_cores > self.cpu_cores {
            return false;
        }
        if requirements.memory.min_bytes > self.memory_bytes {
            return false;
        }
        if requirements.storage.min_bytes > self.storage_bytes {
            return false;
        }
        if let Some(bandwidth_mbps) = requirements.network.bandwidth_mbps {
            let required_bytes = bandwidth_mbps * 1024 * 1024 / 8;
            if required_bytes > self.network_bandwidth {
                return false;
            }
        }
        true
    }
}

pub struct ResourceReservation {
    pub reservation_id: uuid::Uuid,
    pub resources: crate::ResourceRequirements,
}

pub struct RegistrationResponse {
    pub node_id: NodeId,
    pub status: String,
    pub assigned_channels: Vec<String>,
}

pub struct DiscoveryClient {
    pub(crate) connection: Arc<SongbirdConnection>,
    pub(crate) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

#[derive(Default)]
pub struct NodeRegistry {
    pub nodes: HashMap<NodeId, NodeRegistration>,
}

impl NodeRegistry {
    pub fn register(&mut self, registration: NodeRegistration) {
        self.nodes
            .insert(registration.node_id.clone(), registration);
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeRegistration> {
        self.nodes.get(node_id)
    }

    pub fn list_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }
}

pub struct NetworkHealthMonitor {
    pub health_checks: HashMap<NodeId, ConnectionHealth>,
    pub last_check: Option<SystemTime>,
    pub check_interval: Duration,
}

impl Default for NetworkHealthMonitor {
    fn default() -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: timeouts::HEALTH_CHECK_INTERVAL,
        }
    }
}

impl NetworkHealthMonitor {
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: interval,
        }
    }

    pub async fn monitor_health(&mut self) {
        self.last_check = Some(SystemTime::now());
        for (node_id, status) in &mut self.health_checks {
            tracing::debug!("Health check for node {}: {:?}", node_id, status);
        }
    }

    pub fn update_node_health(&mut self, node_id: NodeId, status: ConnectionHealth) {
        let previous = self.health_checks.insert(node_id.clone(), status.clone());
        if let Some(prev) = previous {
            if prev != status {
                tracing::info!(
                    "Node {} health changed: {:?} -> {:?}",
                    node_id,
                    prev,
                    status
                );
            }
        } else {
            tracing::info!("Node {} registered with health: {:?}", node_id, status);
        }
    }

    #[must_use]
    pub fn get_node_health(&self, node_id: &NodeId) -> ConnectionHealth {
        self.health_checks
            .get(node_id)
            .cloned()
            .unwrap_or(ConnectionHealth::Unknown)
    }

    #[must_use]
    pub fn healthy_nodes(&self) -> Vec<NodeId> {
        self.health_checks
            .iter()
            .filter(|(_, status)| **status == ConnectionHealth::Healthy)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.health_checks.remove(node_id);
        tracing::debug!("Removed node {} from health monitoring", node_id);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
    };
    use crate::types::ResourceRequirements;

    fn make_minimal_job(
        cpu: f64,
        memory_bytes: u64,
        storage_bytes: u64,
        bandwidth_mbps: Option<u64>,
    ) -> crate::UniversalJob {
        crate::UniversalJob {
            job_id: uuid::Uuid::new_v4(),
            job_type: None,
            execution_request: toadstool::ExecutionRequest::default(),
            target: crate::ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: cpu,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: memory_bytes,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: storage_bytes,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps,
                    latency_ms: None,
                },
                gpu: None,
            },
            retry_config: crate::types::DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        }
    }

    #[test]
    fn test_available_capacity_can_handle_job_true() {
        let capacity = AvailableCapacity {
            cpu_cores: 4.0,
            memory_bytes: 8 * 1024 * 1024 * 1024,   // 8GB
            storage_bytes: 50 * 1024 * 1024 * 1024, // 50GB
            network_bandwidth: 100 * 1024 * 1024,   // 100 Mbps in bytes/s
        };
        let job = make_minimal_job(
            2.0,
            4 * 1024 * 1024 * 1024,
            10 * 1024 * 1024 * 1024,
            Some(50),
        );
        assert!(capacity.can_handle_job(&job));
    }

    #[test]
    fn test_available_capacity_can_handle_job_false_insufficient_cpu() {
        let capacity = AvailableCapacity {
            cpu_cores: 1.0,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            storage_bytes: 50 * 1024 * 1024 * 1024,
            network_bandwidth: u64::MAX,
        };
        let job = make_minimal_job(4.0, 1, 1, None);
        assert!(!capacity.can_handle_job(&job));
    }

    #[test]
    fn test_available_capacity_can_handle_job_false_insufficient_memory() {
        let capacity = AvailableCapacity {
            cpu_cores: 8.0,
            memory_bytes: 1024 * 1024 * 1024, // 1GB
            storage_bytes: 100 * 1024 * 1024 * 1024,
            network_bandwidth: u64::MAX,
        };
        let job = make_minimal_job(1.0, 8 * 1024 * 1024 * 1024, 1, None); // needs 8GB
        assert!(!capacity.can_handle_job(&job));
    }

    #[test]
    fn test_node_registry_register_and_get() {
        let mut registry = NodeRegistry::default();
        let reg = NodeRegistration {
            node_id: "node-1".to_string(),
            node_type: NodeType::ToadStool,
            capabilities: NodeCapabilities {
                cpu_cores: 4.0,
                memory_gb: 8.0,
                storage_gb: 100.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
            endpoints: vec![toadstool_common::constants::network::default_http_url()],
            protocols: vec!["http".to_string()],
            metadata: NodeMetadata {
                version: "1.0".to_string(),
                build_info: "test".to_string(),
                capabilities: NodeCapabilities {
                    cpu_cores: 4.0,
                    memory_gb: 8.0,
                    storage_gb: 100.0,
                    gpu_count: 0,
                    specialized_hardware: vec![],
                    software_capabilities: vec![],
                },
            },
        };
        registry.register(reg);
        assert!(registry.get_node(&"node-1".to_string()).is_some());
        assert_eq!(registry.list_nodes().len(), 1);
    }

    #[test]
    fn test_network_health_monitor_update_and_get() {
        let mut monitor = NetworkHealthMonitor::default();
        monitor.update_node_health("node-a".to_string(), ConnectionHealth::Healthy);
        assert_eq!(
            monitor.get_node_health(&"node-a".to_string()),
            ConnectionHealth::Healthy
        );
        assert_eq!(monitor.healthy_nodes(), vec!["node-a".to_string()]);
    }

    #[test]
    fn test_network_health_monitor_unknown_for_missing_node() {
        let monitor = NetworkHealthMonitor::default();
        assert_eq!(
            monitor.get_node_health(&"missing".to_string()),
            ConnectionHealth::Unknown
        );
    }

    #[test]
    fn test_network_health_monitor_remove_node() {
        let mut monitor = NetworkHealthMonitor::default();
        monitor.update_node_health("node-x".to_string(), ConnectionHealth::Healthy);
        monitor.remove_node(&"node-x".to_string());
        assert_eq!(
            monitor.get_node_health(&"node-x".to_string()),
            ConnectionHealth::Unknown
        );
    }
}
