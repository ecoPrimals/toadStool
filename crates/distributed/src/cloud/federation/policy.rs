// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::time::Duration;

use toadstool::error::ToadStoolResult;

use crate::cloud::types::{ConnectionStatus, FederationNode, NetworkConnection, NodeConnection};

use super::FederationError;

/// Default heartbeat timeout; members without heartbeat in this interval are considered stale.
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 60;

/// Minimum interval between heartbeats (rate limit).
pub const MIN_HEARTBEAT_INTERVAL_SECS: u64 = 1;

/// Federation member with heartbeat tracking and capability advertisement.
#[derive(Debug, Clone)]
pub struct FederationMember {
    /// Federation node info.
    pub node: FederationNode,
    /// Last heartbeat timestamp (monotonic for timeout checks).
    /// Uses tokio::time::Instant so tests can advance virtual time.
    pub last_heartbeat: tokio::time::Instant,
    /// Advertised capabilities (e.g., "compute", "gpu", "storage").
    pub capabilities: Vec<String>,
}

impl FederationMember {
    pub(super) fn new(node: FederationNode) -> Self {
        let capabilities = node.capabilities.clone();
        Self {
            node,
            last_heartbeat: tokio::time::Instant::now(),
            capabilities,
        }
    }
}

impl super::CloudFederationManager {
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
            return Err(FederationError::AlreadyMember { node_id: node.id }.into());
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

    /// Set heartbeat timeout in seconds.
    #[allow(clippy::missing_const_for_fn)] // Mutates self
    pub fn set_heartbeat_timeout(&mut self, secs: u64) {
        self.heartbeat_timeout_secs = secs;
    }

    /// Return number of registered members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}
