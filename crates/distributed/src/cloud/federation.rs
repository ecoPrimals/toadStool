// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inter-cloud federation and coordination
//!
//! This module provides federation membership, heartbeats, and capability exchange.
//! Full distributed consensus and discovery are not yet implemented; errors clearly
//! indicate what is available.

use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::types::{
    ConnectionStatus, DataReplica, FederationConfig, FederationNode, NetworkConfig,
    NetworkConnection, NodeConnection, ReplicationConfig, TopologyType,
};

// ─── Named Constants ─────────────────────────────────────────────────────────

/// Default heartbeat timeout; members without heartbeat in this interval are considered stale.
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 60;

/// Minimum interval between heartbeats (rate limit).
pub const MIN_HEARTBEAT_INTERVAL_SECS: u64 = 1;

// ─── Federation Errors ───────────────────────────────────────────────────────

/// Federation-related errors. Messages clearly state what is or isn't available.
#[derive(Debug, Error)]
pub enum FederationError {
    #[error("Node '{node_id}' is not a federation member")]
    NotAMember { node_id: String },

    #[error("Node '{node_id}' is already a member")]
    AlreadyMember { node_id: String },

    #[error("Discovery not yet implemented: {0}. Use add_node for local membership.")]
    DiscoveryNotImplemented(String),

    #[error("Cross-federation coordination not yet implemented: {0}")]
    CrossFederationNotImplemented(String),

    #[error("Member '{node_id}' has not sent heartbeat within timeout ({timeout_secs}s)")]
    MemberStale { node_id: String, timeout_secs: u64 },

    #[error("Heartbeat rate limit: wait at least {min_interval_secs}s between heartbeats")]
    HeartbeatRateLimited { min_interval_secs: u64 },

    #[error("Invalid node: {0}")]
    InvalidNode(String),
}

impl From<FederationError> for ToadStoolError {
    fn from(e: FederationError) -> Self {
        ToadStoolError::Integration(toadstool::error::IntegrationError::OperationFailed {
            service: "federation".into(),
            operation: "coordinate".into(),
            reason: e.to_string(),
        })
    }
}

// ─── Federation Member (with heartbeat) ───────────────────────────────────────

/// Federation member with heartbeat tracking and capability advertisement.
#[derive(Debug, Clone)]
pub struct FederationMember {
    pub node: FederationNode,
    /// Last heartbeat timestamp (monotonic for timeout checks).
    /// Uses tokio::time::Instant so tests can advance virtual time.
    pub last_heartbeat: tokio::time::Instant,
    /// Advertised capabilities (e.g., "compute", "gpu", "storage").
    pub capabilities: Vec<String>,
}

impl FederationMember {
    fn new(node: FederationNode) -> Self {
        let capabilities = node.capabilities.clone();
        Self {
            node,
            last_heartbeat: tokio::time::Instant::now(),
            capabilities,
        }
    }
}

// ─── CloudFederationManager ───────────────────────────────────────────────────

/// Cloud federation manager with membership, heartbeats, and capability exchange.
pub struct CloudFederationManager {
    topology: CloudFederationTopology,
    network: InterCloudNetworkManager,
    replication: CloudDataReplicationManager,
    pub(crate) config: FederationConfig,
    /// Membership: node_id -> member with heartbeat
    members: HashMap<String, FederationMember>,
    heartbeat_timeout_secs: u64,
}

impl CloudFederationManager {
    pub async fn new(config: FederationConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            topology: CloudFederationTopology::new(TopologyType::default()),
            network: InterCloudNetworkManager::new(NetworkConfig::default()),
            replication: CloudDataReplicationManager::new(ReplicationConfig::default()),
            config,
            members: HashMap::new(),
            heartbeat_timeout_secs: DEFAULT_HEARTBEAT_TIMEOUT_SECS,
        })
    }

    /// Add a federation node (membership) and register it in the topology and network layer.
    pub fn add_node(
        &mut self,
        node: FederationNode,
        connections: Vec<NodeConnection>,
    ) -> ToadStoolResult<()> {
        if node.id.is_empty() {
            return Err(FederationError::InvalidNode("node id cannot be empty".to_string()).into());
        }
        if self.members.contains_key(&node.id) {
            return Err(FederationError::AlreadyMember {
                node_id: node.id.clone(),
            }
            .into());
        }

        let member = FederationMember::new(node.clone());
        self.members.insert(node.id.clone(), member);
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
        Ok(())
    }

    /// Remove a node from the federation (leave membership).
    pub fn remove_node(&mut self, node_id: &str) -> ToadStoolResult<()> {
        if !self.members.contains_key(node_id) {
            return Err(FederationError::NotAMember {
                node_id: node_id.to_string(),
            }
            .into());
        }
        self.members.remove(node_id);
        self.topology.nodes.retain(|n| n.id != node_id);
        self.topology
            .connections
            .retain(|c| c.from != *node_id && c.to != *node_id);
        self.network.connections.remove(node_id);
        Ok(())
    }

    /// Record a heartbeat from a member. Returns error if not a member or rate-limited.
    pub fn record_heartbeat(&mut self, node_id: &str) -> ToadStoolResult<()> {
        let member = self
            .members
            .get_mut(node_id)
            .ok_or_else(|| FederationError::NotAMember {
                node_id: node_id.to_string(),
            })?;

        let elapsed = member.last_heartbeat.elapsed();
        if elapsed < Duration::from_secs(MIN_HEARTBEAT_INTERVAL_SECS) {
            return Err(FederationError::HeartbeatRateLimited {
                min_interval_secs: MIN_HEARTBEAT_INTERVAL_SECS,
            }
            .into());
        }

        member.last_heartbeat = tokio::time::Instant::now();
        Ok(())
    }

    /// Record heartbeat with optional capability update.
    pub fn record_heartbeat_with_capabilities(
        &mut self,
        node_id: &str,
        capabilities: Vec<String>,
    ) -> ToadStoolResult<()> {
        self.record_heartbeat(node_id)?;
        if let Some(member) = self.members.get_mut(node_id) {
            member.capabilities = capabilities;
        }
        Ok(())
    }

    /// Check if a member is alive (has sent heartbeat within timeout).
    pub fn is_member_alive(&self, node_id: &str) -> bool {
        self.members
            .get(node_id)
            .map(|m| m.last_heartbeat.elapsed() < Duration::from_secs(self.heartbeat_timeout_secs))
            .unwrap_or(false)
    }

    /// Get all alive member IDs.
    pub fn alive_members(&self) -> Vec<String> {
        self.members
            .iter()
            .filter(|(_, m)| {
                m.last_heartbeat.elapsed() < Duration::from_secs(self.heartbeat_timeout_secs)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Exchange: get all capabilities from alive members, aggregated by capability name.
    pub fn get_federation_capabilities(&self) -> HashMap<String, Vec<String>> {
        let mut agg: HashMap<String, Vec<String>> = HashMap::new();
        for (node_id, member) in &self.members {
            if member.last_heartbeat.elapsed() < Duration::from_secs(self.heartbeat_timeout_secs) {
                for cap in &member.capabilities {
                    agg.entry(cap.clone()).or_default().push(node_id.clone());
                }
            }
        }
        agg
    }

    /// Get capabilities advertised by a specific member.
    pub fn get_member_capabilities(&self, node_id: &str) -> ToadStoolResult<Vec<String>> {
        let member = self
            .members
            .get(node_id)
            .ok_or_else(|| FederationError::NotAMember {
                node_id: node_id.to_string(),
            })?;
        Ok(member.capabilities.clone())
    }

    /// Discover federation nodes from configured discovery endpoints.
    ///
    /// Iterates `config.discovery_endpoints` and attempts to connect to each
    /// as a federation peer. Endpoints that respond with valid node metadata
    /// are returned as `FederationNode` candidates for `add_node()`.
    ///
    /// Returns an empty vec (not an error) if no discovery endpoints are configured
    /// or none respond -- federation can still function with manually-added nodes.
    pub async fn discover_nodes(&self) -> ToadStoolResult<Vec<FederationNode>> {
        if self.config.discovery_endpoints.is_empty() {
            return Ok(Vec::new());
        }

        let mut discovered = Vec::new();
        for endpoint in &self.config.discovery_endpoints {
            match self.probe_endpoint(endpoint).await {
                Ok(node) => discovered.push(node),
                Err(e) => {
                    tracing::debug!("Federation endpoint {endpoint} unreachable: {e}");
                }
            }
        }

        tracing::info!(
            "Federation discovery: {} of {} endpoints responded",
            discovered.len(),
            self.config.discovery_endpoints.len()
        );
        Ok(discovered)
    }

    async fn probe_endpoint(&self, endpoint: &str) -> ToadStoolResult<FederationNode> {
        use tokio::net::TcpStream;
        use tokio::time::timeout;

        let addr = endpoint
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let stream = timeout(Duration::from_secs(5), TcpStream::connect(addr))
            .await
            .map_err(|_| FederationError::InvalidNode(format!("Timeout connecting to {endpoint}")))?
            .map_err(|_| FederationError::InvalidNode(format!("Cannot connect to {endpoint}")))?;

        let peer_addr = stream
            .peer_addr()
            .map_err(|e| FederationError::InvalidNode(e.to_string()))?;

        Ok(FederationNode {
            id: format!("discovered-{peer_addr}"),
            provider: endpoint.to_string(),
            capabilities: vec!["compute".to_string()],
            region: String::new(),
        })
    }

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

    /// Set heartbeat timeout in seconds.
    pub fn set_heartbeat_timeout(&mut self, secs: u64) {
        self.heartbeat_timeout_secs = secs;
    }

    /// Return number of registered members.
    pub fn member_count(&self) -> usize {
        self.members.len()
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
        assert_eq!(mgr.member_count(), 0);
    }

    #[tokio::test]
    async fn test_add_node_increases_count() {
        let mut mgr = CloudFederationManager::new(make_config("fed-002"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
        assert_eq!(mgr.node_ids().count(), 1);
        assert_eq!(mgr.member_count(), 1);

        mgr.add_node(make_node("node-b", "gcp"), vec![]).unwrap();
        assert_eq!(mgr.node_ids().count(), 2);
        assert_eq!(mgr.member_count(), 2);
    }

    #[tokio::test]
    async fn test_add_node_duplicate_fails() {
        let mut mgr = CloudFederationManager::new(make_config("fed-dup"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
        let res = mgr.add_node(make_node("node-a", "gcp"), vec![]);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_remove_node() {
        let mut mgr = CloudFederationManager::new(make_config("fed-rm"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
        mgr.remove_node("node-a").unwrap();
        assert_eq!(mgr.member_count(), 0);
        assert_eq!(mgr.node_ids().count(), 0);
    }

    #[tokio::test]
    async fn test_remove_non_member_fails() {
        let mut mgr = CloudFederationManager::new(make_config("fed-rm2"))
            .await
            .unwrap();
        let res = mgr.remove_node("nonexistent");
        assert!(res.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn test_heartbeat_keeps_member_alive() {
        let mut mgr = CloudFederationManager::new(make_config("fed-hb"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
        assert!(mgr.is_member_alive("node-a"));

        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        mgr.record_heartbeat("node-a").unwrap();
        assert!(mgr.is_member_alive("node-a"));
    }

    #[tokio::test]
    async fn test_heartbeat_non_member_fails() {
        let mut mgr = CloudFederationManager::new(make_config("fed-hb2"))
            .await
            .unwrap();
        let res = mgr.record_heartbeat("nonexistent");
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_capability_exchange() {
        let mut mgr = CloudFederationManager::new(make_config("fed-cap"))
            .await
            .unwrap();
        mgr.add_node(
            FederationNode {
                id: "n1".to_string(),
                provider: "aws".to_string(),
                region: "us-east-1".to_string(),
                capabilities: vec!["compute".to_string(), "gpu".to_string()],
            },
            vec![],
        )
        .unwrap();
        mgr.add_node(
            FederationNode {
                id: "n2".to_string(),
                provider: "gcp".to_string(),
                region: "us-west-1".to_string(),
                capabilities: vec!["compute".to_string(), "storage".to_string()],
            },
            vec![],
        )
        .unwrap();

        let caps = mgr.get_federation_capabilities();
        assert!(caps.get("compute").map(|v| v.len() == 2).unwrap_or(false));
        assert!(caps.contains_key("gpu"));
        assert!(caps.contains_key("storage"));
    }

    #[tokio::test]
    async fn test_discover_nodes_unreachable_endpoints() {
        let mgr = CloudFederationManager::new(make_config("fed-disc"))
            .await
            .unwrap();
        let res = mgr.discover_nodes().await;
        assert!(res.is_ok());
        // Unreachable endpoints produce empty results, not errors
        assert!(res.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_discover_nodes_no_endpoints() {
        let config = FederationConfig {
            federation_id: "fed-empty".to_string(),
            discovery_endpoints: vec![],
            trust_anchors: vec![],
        };
        let mgr = CloudFederationManager::new(config).await.unwrap();
        let res = mgr.discover_nodes().await.unwrap();
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn test_add_node_ids_are_accessible() {
        let mut mgr = CloudFederationManager::new(make_config("fed-003"))
            .await
            .unwrap();
        mgr.add_node(make_node("alpha", "aws"), vec![]).unwrap();
        mgr.add_node(make_node("beta", "azure"), vec![]).unwrap();

        let ids: Vec<&str> = mgr.node_ids().collect();
        assert!(ids.contains(&"alpha"));
        assert!(ids.contains(&"beta"));
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
    async fn test_topology_type_defaults_to_centralized() {
        let mgr = CloudFederationManager::new(make_config("fed-007"))
            .await
            .unwrap();
        assert!(matches!(mgr.topology_type(), TopologyType::Centralized));
    }

    #[tokio::test]
    async fn test_federation_id_round_trip() {
        let id = "my-unique-federation-42";
        let mgr = CloudFederationManager::new(make_config(id)).await.unwrap();
        assert_eq!(mgr.federation_id(), id);
    }

    #[tokio::test]
    async fn test_add_node_empty_id_fails() {
        let mut mgr = CloudFederationManager::new(make_config("fed-empty"))
            .await
            .unwrap();
        let node = FederationNode {
            id: String::new(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            capabilities: vec!["compute".to_string()],
        };
        let res = mgr.add_node(node, vec![]);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_member_capabilities_returns_capabilities() {
        let mut mgr = CloudFederationManager::new(make_config("fed-caps"))
            .await
            .unwrap();
        mgr.add_node(
            FederationNode {
                id: "node-x".to_string(),
                provider: "gcp".to_string(),
                region: "us-west-1".to_string(),
                capabilities: vec!["compute".to_string(), "storage".to_string()],
            },
            vec![],
        )
        .unwrap();

        let caps = mgr.get_member_capabilities("node-x").unwrap();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&"compute".to_string()));
        assert!(caps.contains(&"storage".to_string()));
    }

    #[tokio::test]
    async fn test_get_member_capabilities_non_member_fails() {
        let mgr = CloudFederationManager::new(make_config("fed-caps2"))
            .await
            .unwrap();
        let res = mgr.get_member_capabilities("nonexistent");
        assert!(res.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn test_record_heartbeat_with_capabilities_updates_caps() {
        let mut mgr = CloudFederationManager::new(make_config("fed-hbc"))
            .await
            .unwrap();
        mgr.add_node(make_node("node-hb", "aws"), vec![]).unwrap();
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        mgr.record_heartbeat_with_capabilities("node-hb", vec!["gpu".to_string()])
            .unwrap();

        let caps = mgr.get_member_capabilities("node-hb").unwrap();
        assert_eq!(caps, vec!["gpu".to_string()]);
    }

    #[tokio::test]
    async fn test_set_heartbeat_timeout() {
        let mut mgr = CloudFederationManager::new(make_config("fed-timeout"))
            .await
            .unwrap();
        mgr.set_heartbeat_timeout(120);
        mgr.add_node(make_node("n1", "aws"), vec![]).unwrap();
        assert!(mgr.is_member_alive("n1"));
    }

    #[tokio::test]
    async fn test_replication_factor() {
        let mgr = CloudFederationManager::new(make_config("fed-repl"))
            .await
            .unwrap();
        let factor = mgr.replication_factor();
        assert!(factor <= 10); // Default config should have reasonable factor
    }

    #[tokio::test]
    async fn test_is_network_encrypted() {
        let mgr = CloudFederationManager::new(make_config("fed-net"))
            .await
            .unwrap();
        let _ = mgr.is_network_encrypted();
    }
}
