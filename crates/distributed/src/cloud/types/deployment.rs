// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::common::distribution::DistributionStrategy as CommonDistributionStrategy;

/// Handle for a cloud job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudJobHandle {
    pub job_id: Uuid,
    pub provider_job_id: String,
    pub provider_name: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,
}

/// Cloud job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudJobStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
}

/// Scale configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleConfig {
    pub target_replicas: Option<u32>,
    pub cpu_scale_factor: Option<f64>,
    pub memory_scale_factor: Option<f64>,
}

/// Deployment strategy options
#[derive(Debug, Clone)]
pub enum DeploymentStrategy {
    SingleCloud {
        provider_name: String,
    },
    MultiCloud {
        providers: Vec<String>,
        distribution: MultiCloudDistribution,
    },
    HybridCloudBurst {
        primary: String,
        burst_providers: Vec<String>,
    },
    FederatedDeployment {
        federation_nodes: Vec<String>,
    },
}

/// Cloud deployment result
#[derive(Debug, Clone)]
pub enum CloudDeploymentResult {
    Single {
        provider: String,
        handle: CloudJobHandle,
    },
    Multi {
        handles: HashMap<String, CloudJobHandle>,
    },
    Federated {
        deployment: FederatedDeployment,
    },
}

/// Multi-cloud distribution configuration
#[derive(Debug, Clone)]
pub struct MultiCloudDistribution {
    pub providers: Vec<String>,
    pub strategy: DistributionStrategy,
}

/// Cloud distribution strategy (re-exported from common for backward compatibility)
pub type DistributionStrategy = CommonDistributionStrategy;

/// Burst distribution configuration
#[derive(Debug, Clone)]
pub struct BurstDistribution {
    pub providers: Vec<String>,
    pub primary_provider: String,
}

/// Federated deployment configuration
#[derive(Debug, Clone)]
pub struct FederatedDeployment {
    pub federation_id: Uuid,
    pub nodes: Vec<String>,
    pub coordination_endpoint: String,
}

/// Topology type for federation
#[derive(Debug, Clone, Default)]
pub enum TopologyType {
    #[default]
    Centralized,
    Distributed,
    Mesh,
    Hierarchical,
}

/// Federation node information
#[derive(Debug, Clone, Default)]
pub struct FederationNode {
    pub id: String,
    pub provider: String,
    pub region: String,
    pub capabilities: Vec<String>,
}

/// Connection between federation nodes
#[derive(Debug, Clone, Default)]
pub struct NodeConnection {
    pub from: String,
    pub to: String,
    pub latency: f64,
    pub bandwidth: f64,
}

/// Network connection status
#[derive(Debug, Clone, Default)]
pub struct NetworkConnection {
    pub id: String,
    pub provider: String,
    pub status: ConnectionStatus,
}

/// Connection status enum
#[derive(Debug, Clone, Default)]
pub enum ConnectionStatus {
    #[default]
    Active,
    Inactive,
    Error,
}

/// Data replica information
#[derive(Debug, Clone, Default)]
pub struct DataReplica {
    pub id: String,
    pub location: String,
    pub status: ReplicaStatus,
}

/// Replica status enum
#[derive(Debug, Clone, Default)]
pub enum ReplicaStatus {
    #[default]
    Synced,
    Syncing,
    OutOfSync,
}

/// Replication configuration
#[derive(Debug, Clone, Default)]
pub struct ReplicationConfig {
    pub factor: u32,
    pub consistency: ConsistencyLevel,
}

/// Consistency level for replication
#[derive(Debug, Clone, Default)]
pub enum ConsistencyLevel {
    #[default]
    Strong,
    Eventual,
    Weak,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_type_default() {
        let topo = TopologyType::default();
        assert!(matches!(topo, TopologyType::Centralized));
    }

    #[test]
    fn test_federation_node_default() {
        let node = FederationNode::default();
        assert!(node.id.is_empty());
        assert!(node.capabilities.is_empty());
    }

    #[test]
    fn test_replica_status_default() {
        let status = ReplicaStatus::default();
        assert!(matches!(status, ReplicaStatus::Synced));
    }

    #[test]
    fn test_connection_status_default() {
        let status = ConnectionStatus::default();
        assert!(matches!(status, ConnectionStatus::Active));
    }

    #[test]
    fn test_consistency_level_default() {
        let level = ConsistencyLevel::default();
        assert!(matches!(level, ConsistencyLevel::Strong));
    }

    #[test]
    fn test_replication_config_default() {
        let config = ReplicationConfig::default();
        assert_eq!(config.factor, 0);
        assert!(matches!(config.consistency, ConsistencyLevel::Strong));
    }

    #[test]
    fn test_cloud_job_status_serialization() {
        let status = CloudJobStatus::Running;
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.to_lowercase().contains("running"));
        let parsed: CloudJobStatus = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(parsed, CloudJobStatus::Running));
    }

    #[test]
    fn test_cloud_job_status_failed_variant() {
        let status = CloudJobStatus::Failed {
            error: "test error".to_string(),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_node_connection_default() {
        let conn = NodeConnection::default();
        assert!(conn.from.is_empty());
        assert!(conn.to.is_empty());
        assert_eq!(conn.latency, 0.0);
    }

    #[test]
    fn test_data_replica_default() {
        let replica = DataReplica::default();
        assert!(replica.id.is_empty());
        assert!(matches!(replica.status, ReplicaStatus::Synced));
    }
}
