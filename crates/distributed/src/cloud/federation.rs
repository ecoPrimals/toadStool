//! Inter-cloud federation and coordination
//!
//! This module contains the federation manager and related networking functionality.

use std::collections::HashMap;
use toadstool::error::ToadStoolResult;

use super::types::{
    ConnectionStatus, DataReplica, FederationConfig, FederationNode, NetworkConfig,
    NetworkConnection, NodeConnection, ReplicationConfig, TopologyType,
};

/// Cloud federation manager
///
/// Coordinates inter-cloud topology, networking, and data replication.
pub struct CloudFederationManager {
    topology: CloudFederationTopology,
    network: InterCloudNetworkManager,
    replication: CloudDataReplicationManager,
    pub(crate) config: FederationConfig,
}

impl CloudFederationManager {
    pub async fn new(config: FederationConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            topology: CloudFederationTopology::new(TopologyType::default()),
            network: InterCloudNetworkManager::new(NetworkConfig::default()),
            replication: CloudDataReplicationManager::new(ReplicationConfig::default()),
            config,
        })
    }

    /// Add a federation node and register it in the topology and network layer
    pub fn add_node(&mut self, node: FederationNode, connections: Vec<NodeConnection>) {
        self.topology.nodes.push(node.clone());
        self.topology.connections.extend(connections);
        self.network
            .connections
            .entry(node.id.clone())
            .or_insert_with(|| NetworkConnection {
                id: node.id,
                provider: node.provider,
                status: ConnectionStatus::Active,
            });
    }

    /// Register a data replica managed by this federation
    pub fn register_replica(&mut self, replica: DataReplica) {
        self.replication
            .replicas
            .insert(replica.id.clone(), replica);
    }

    /// Return the IDs of all nodes in this federation
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.topology.nodes.iter().map(|n| n.id.as_str())
    }

    /// Return the number of tracked data replicas
    pub fn replica_count(&self) -> usize {
        self.replication.replicas.len()
    }

    /// Return the federation topology type
    pub fn topology_type(&self) -> &TopologyType {
        self.topology.topology_type()
    }

    /// Return whether inter-node links use encryption
    pub fn is_network_encrypted(&self) -> bool {
        self.network.is_encrypted()
    }

    /// Return the replication factor configured for this federation
    pub fn replication_factor(&self) -> u32 {
        self.replication.replication_factor()
    }

    /// Return the federation ID
    pub fn federation_id(&self) -> &str {
        &self.config.federation_id
    }
}

struct CloudFederationTopology {
    topology_type: TopologyType,
    nodes: Vec<FederationNode>,
    connections: Vec<NodeConnection>,
}

impl CloudFederationTopology {
    fn new(topology_type: TopologyType) -> Self {
        Self {
            topology_type,
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    fn topology_type(&self) -> &TopologyType {
        &self.topology_type
    }
}

struct InterCloudNetworkManager {
    network_config: NetworkConfig,
    connections: HashMap<String, NetworkConnection>,
}

impl InterCloudNetworkManager {
    fn new(network_config: NetworkConfig) -> Self {
        Self {
            network_config,
            connections: HashMap::new(),
        }
    }

    fn is_encrypted(&self) -> bool {
        self.network_config.encryption
    }
}

struct CloudDataReplicationManager {
    replication_config: ReplicationConfig,
    replicas: HashMap<String, DataReplica>,
}

impl CloudDataReplicationManager {
    fn new(replication_config: ReplicationConfig) -> Self {
        Self {
            replication_config,
            replicas: HashMap::new(),
        }
    }

    fn replication_factor(&self) -> u32 {
        self.replication_config.factor
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::ReplicaStatus;

    fn make_config(id: &str) -> FederationConfig {
        FederationConfig {
            federation_id: id.to_string(),
            discovery_endpoints: vec!["https://discovery.example.com".to_string()],
            trust_anchors: vec!["anchor-1".to_string()],
        }
    }

    fn make_node(id: &str, provider: &str) -> FederationNode {
        FederationNode {
            id: id.to_string(),
            provider: provider.to_string(),
            region: "us-east-1".to_string(),
            capabilities: vec!["compute".to_string()],
        }
    }

    fn make_replica(id: &str, location: &str) -> DataReplica {
        DataReplica {
            id: id.to_string(),
            location: location.to_string(),
            status: ReplicaStatus::Synced,
        }
    }

    #[tokio::test]
    async fn test_new_federation_manager_is_empty() {
        let mgr = CloudFederationManager::new(make_config("fed-001"))
            .await
            .unwrap();
        assert_eq!(mgr.federation_id(), "fed-001");
        assert_eq!(mgr.node_ids().count(), 0);
        assert_eq!(mgr.replica_count(), 0);
    }

    #[tokio::test]
    async fn test_add_node_increases_count() {
        let mut mgr = CloudFederationManager::new(make_config("fed-002"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-a", "aws"), vec![]);
        assert_eq!(mgr.node_ids().count(), 1);

        mgr.add_node(make_node("node-b", "gcp"), vec![]);
        assert_eq!(mgr.node_ids().count(), 2);
    }

    #[tokio::test]
    async fn test_add_node_ids_are_accessible() {
        let mut mgr = CloudFederationManager::new(make_config("fed-003"))
            .await
            .unwrap();
        mgr.add_node(make_node("alpha", "aws"), vec![]);
        mgr.add_node(make_node("beta", "azure"), vec![]);

        let ids: Vec<&str> = mgr.node_ids().collect();
        assert!(ids.contains(&"alpha"));
        assert!(ids.contains(&"beta"));
    }

    #[tokio::test]
    async fn test_add_node_with_connections() {
        let mut mgr = CloudFederationManager::new(make_config("fed-004"))
            .await
            .unwrap();
        let conn = NodeConnection {
            from: "node-a".to_string(),
            to: "node-b".to_string(),
            latency: 5.0,
            bandwidth: 1000.0,
        };
        mgr.add_node(make_node("node-a", "aws"), vec![conn]);
        assert_eq!(mgr.node_ids().count(), 1);
    }

    #[tokio::test]
    async fn test_register_replica_increases_count() {
        let mut mgr = CloudFederationManager::new(make_config("fed-005"))
            .await
            .unwrap();
        assert_eq!(mgr.replica_count(), 0);

        mgr.register_replica(make_replica("replica-1", "us-east-1"));
        assert_eq!(mgr.replica_count(), 1);

        mgr.register_replica(make_replica("replica-2", "eu-west-1"));
        assert_eq!(mgr.replica_count(), 2);
    }

    #[tokio::test]
    async fn test_register_replica_overwrites_same_id() {
        let mut mgr = CloudFederationManager::new(make_config("fed-006"))
            .await
            .unwrap();
        mgr.register_replica(make_replica("r-1", "us-east-1"));
        mgr.register_replica(make_replica("r-1", "eu-west-1")); // same id, different location
        assert_eq!(mgr.replica_count(), 1, "same-id replica should overwrite");
    }

    #[tokio::test]
    async fn test_topology_type_defaults_to_centralized() {
        let mgr = CloudFederationManager::new(make_config("fed-007"))
            .await
            .unwrap();
        assert!(matches!(mgr.topology_type(), TopologyType::Centralized));
    }

    #[tokio::test]
    async fn test_is_network_encrypted_default_false() {
        let mgr = CloudFederationManager::new(make_config("fed-008"))
            .await
            .unwrap();
        assert!(
            !mgr.is_network_encrypted(),
            "default NetworkConfig::encryption is false"
        );
    }

    #[tokio::test]
    async fn test_replication_factor_default_zero() {
        let mgr = CloudFederationManager::new(make_config("fed-009"))
            .await
            .unwrap();
        assert_eq!(
            mgr.replication_factor(),
            0,
            "default ReplicationConfig::factor is 0"
        );
    }

    #[tokio::test]
    async fn test_federation_id_round_trip() {
        let id = "my-unique-federation-42";
        let mgr = CloudFederationManager::new(make_config(id)).await.unwrap();
        assert_eq!(mgr.federation_id(), id);
    }
}
