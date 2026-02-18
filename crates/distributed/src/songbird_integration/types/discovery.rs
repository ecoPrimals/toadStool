//! Service discovery and endpoint types

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    pub last_check: Option<DateTime<Utc>>,
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
        self.last_check = Some(chrono::Utc::now());
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
