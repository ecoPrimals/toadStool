// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::common::distribution::DistributionStrategy as CommonDistributionStrategy;

/// Handle for a cloud job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudJobHandle {
    /// Job identifier.
    pub job_id: Uuid,
    /// Provider-specific job ID.
    pub provider_job_id: String,
    /// Cloud provider name.
    pub provider_name: String,
    /// Creation timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,
}

/// Cloud job lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudJobStatus {
    /// Job queued.
    Pending,
    /// Job running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed with error.
    Failed { error: String },
    /// Job cancelled.
    Cancelled,
}

/// Scale configuration for auto-scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleConfig {
    /// Target replica count.
    pub target_replicas: Option<u32>,
    /// CPU scale factor.
    pub cpu_scale_factor: Option<f64>,
    /// Memory scale factor.
    pub memory_scale_factor: Option<f64>,
}

/// Deployment strategy for cloud workload placement.
#[derive(Debug, Clone)]
pub enum DeploymentStrategy {
    /// Single cloud provider.
    SingleCloud {
        /// Provider name.
        provider_name: String,
    },
    /// Multi-cloud distribution.
    MultiCloud {
        /// Provider names.
        providers: Vec<String>,
        /// Distribution config.
        distribution: MultiCloudDistribution,
    },
    /// Hybrid with burst to cloud.
    HybridCloudBurst {
        /// Primary (on-prem) provider.
        primary: String,
        /// Burst providers.
        burst_providers: Vec<String>,
    },
    /// Federated deployment across nodes.
    FederatedDeployment {
        /// Federation node IDs.
        federation_nodes: Vec<String>,
    },
}

/// Result of cloud deployment.
#[derive(Debug, Clone)]
pub enum CloudDeploymentResult {
    /// Single-provider deployment.
    Single {
        /// Provider name.
        provider: String,
        /// Job handle.
        handle: CloudJobHandle,
    },
    /// Multi-provider deployment.
    Multi {
        /// Per-provider handles.
        handles: HashMap<String, CloudJobHandle>,
    },
    /// Federated deployment.
    Federated {
        /// Federated deployment config.
        deployment: FederatedDeployment,
    },
}

/// Multi-cloud distribution configuration.
#[derive(Debug, Clone)]
pub struct MultiCloudDistribution {
    /// Provider names.
    pub providers: Vec<String>,
    /// Distribution strategy.
    pub strategy: DistributionStrategy,
}

/// Cloud distribution strategy (re-exported from common for backward compatibility).
pub type DistributionStrategy = CommonDistributionStrategy;

/// Burst distribution configuration for hybrid cloud.
#[derive(Debug, Clone)]
pub struct BurstDistribution {
    /// Burst providers.
    pub providers: Vec<String>,
    /// Primary provider.
    pub primary_provider: String,
}

/// Federated deployment configuration.
#[derive(Debug, Clone)]
pub struct FederatedDeployment {
    /// Federation identifier.
    pub federation_id: Uuid,
    /// Node IDs.
    pub nodes: Vec<String>,
    /// Coordination endpoint URL.
    pub coordination_endpoint: String,
}

/// Topology type for federation.
#[derive(Debug, Clone, Default)]
pub enum TopologyType {
    /// Central coordinator.
    #[default]
    Centralized,
    /// Distributed coordination.
    Distributed,
    /// Full mesh.
    Mesh,
    /// Hierarchical structure.
    Hierarchical,
}

/// Federation node information.
#[derive(Debug, Clone, Default)]
pub struct FederationNode {
    /// Node ID.
    pub id: String,
    /// Cloud provider.
    pub provider: String,
    /// Region.
    pub region: String,
    /// Node capabilities.
    pub capabilities: Vec<String>,
}

/// Connection between federation nodes.
#[derive(Debug, Clone, Default)]
pub struct NodeConnection {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Latency in ms.
    pub latency: f64,
    /// Bandwidth in Gbps.
    pub bandwidth: f64,
}

/// Network connection status.
#[derive(Debug, Clone, Default)]
pub struct NetworkConnection {
    /// Connection ID.
    pub id: String,
    /// Provider name.
    pub provider: String,
    /// Connection status.
    pub status: ConnectionStatus,
}

/// Connection status for federation links.
#[derive(Debug, Clone, Default)]
pub enum ConnectionStatus {
    /// Connection active.
    #[default]
    Active,
    /// Connection inactive.
    Inactive,
    /// Connection error.
    Error,
}

/// Data replica information for replication.
#[derive(Debug, Clone, Default)]
pub struct DataReplica {
    /// Replica ID.
    pub id: String,
    /// Replica location.
    pub location: String,
    /// Sync status.
    pub status: ReplicaStatus,
}

/// Replica sync status.
#[derive(Debug, Clone, Default)]
pub enum ReplicaStatus {
    /// Replica in sync.
    #[default]
    Synced,
    /// Replica syncing.
    Syncing,
    /// Replica out of sync.
    OutOfSync,
}

/// Replication configuration.
#[derive(Debug, Clone, Default)]
pub struct ReplicationConfig {
    /// Replication factor.
    pub factor: u32,
    /// Consistency level.
    pub consistency: ConsistencyLevel,
}

/// Consistency level for distributed replication.
#[derive(Debug, Clone, Default)]
pub enum ConsistencyLevel {
    /// Strong consistency.
    #[default]
    Strong,
    /// Eventual consistency.
    Eventual,
    /// Weak consistency.
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
        assert!((conn.latency - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_data_replica_default() {
        let replica = DataReplica::default();
        assert!(replica.id.is_empty());
        assert!(matches!(replica.status, ReplicaStatus::Synced));
    }
}
