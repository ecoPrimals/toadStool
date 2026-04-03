// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use crate::cloud::types::{
    DataReplica, FederationNode, NetworkConfig, NetworkConnection, NodeConnection,
    ReplicationConfig, TopologyType,
};

pub(super) struct CloudFederationTopology {
    pub(super) topology_type: TopologyType,
    pub(super) nodes: Vec<FederationNode>,
    pub(super) connections: Vec<NodeConnection>,
}

impl CloudFederationTopology {
    pub(super) const fn new(topology_type: TopologyType) -> Self {
        Self {
            topology_type,
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub(super) const fn topology_type(&self) -> &TopologyType {
        &self.topology_type
    }
}

pub(super) struct InterCloudNetworkManager {
    network_config: NetworkConfig,
    pub(super) connections: HashMap<String, NetworkConnection>,
}

impl InterCloudNetworkManager {
    pub(super) fn new(network_config: NetworkConfig) -> Self {
        Self {
            network_config,
            connections: HashMap::new(),
        }
    }

    pub(super) const fn is_encrypted(&self) -> bool {
        self.network_config.encryption
    }
}

pub(super) struct CloudDataReplicationManager {
    replication_config: ReplicationConfig,
    pub(super) replicas: HashMap<String, DataReplica>,
}

impl CloudDataReplicationManager {
    pub(super) fn new(replication_config: ReplicationConfig) -> Self {
        Self {
            replication_config,
            replicas: HashMap::new(),
        }
    }

    pub(super) const fn replication_factor(&self) -> u32 {
        self.replication_config.factor
    }
}

impl super::CloudFederationManager {
    /// Register a data replica managed by this federation
    pub fn register_replica(&mut self, replica: DataReplica) {
        self.replication
            .replicas
            .insert(replica.id.clone(), replica);
    }

    /// Return the IDs of all nodes in this federation (includes stale members).
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.topology.nodes.iter().map(|n| n.id.as_str())
    }

    /// Return the number of tracked data replicas
    pub fn replica_count(&self) -> usize {
        self.replication.replicas.len()
    }

    /// Return the federation topology type
    pub const fn topology_type(&self) -> &TopologyType {
        self.topology.topology_type()
    }

    /// Return whether inter-node links use encryption
    pub const fn is_network_encrypted(&self) -> bool {
        self.network.is_encrypted()
    }

    /// Return the replication factor configured for this federation
    pub const fn replication_factor(&self) -> u32 {
        self.replication.replication_factor()
    }

    /// Return the federation ID
    pub fn federation_id(&self) -> &str {
        &self.config.federation_id
    }
}
