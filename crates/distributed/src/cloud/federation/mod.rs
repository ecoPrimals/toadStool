// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inter-cloud federation and coordination
//!
//! This module provides federation membership, heartbeats, and capability exchange.
//! Full distributed consensus and discovery are not yet implemented; errors clearly
//! indicate what is available.

use std::collections::HashMap;

use thiserror::Error;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::cloud::types::{FederationConfig, NetworkConfig, ReplicationConfig, TopologyType};

pub mod discovery;
pub mod policy;
mod state;

#[allow(unused_imports, reason = "MIN_HEARTBEAT_INTERVAL_SECS re-exported for downstream crates")]
pub use policy::{DEFAULT_HEARTBEAT_TIMEOUT_SECS, FederationMember, MIN_HEARTBEAT_INTERVAL_SECS};

// ─── Federation Errors ───────────────────────────────────────────────────────

/// Federation-related errors. Messages clearly state what is or isn't available.
#[derive(Debug, Error)]
pub enum FederationError {
    /// Node is not a federation member.
    #[error("Node '{node_id}' is not a federation member")]
    NotAMember {
        /// Node ID.
        node_id: String,
    },
    /// Node is already a member.
    #[error("Node '{node_id}' is already a member")]
    AlreadyMember {
        /// Node ID.
        node_id: String,
    },
    /// Discovery not yet implemented.
    #[error("Discovery not yet implemented: {0}. Use add_node for local membership.")]
    DiscoveryNotImplemented(String),
    /// Cross-federation coordination not yet implemented.
    #[error("Cross-federation coordination not yet implemented: {0}")]
    CrossFederationNotImplemented(String),
    /// Member has not sent heartbeat within timeout.
    #[error("Member '{node_id}' has not sent heartbeat within timeout ({timeout_secs}s)")]
    MemberStale {
        /// Node ID.
        node_id: String,
        /// Timeout in seconds.
        timeout_secs: u64,
    },
    /// Heartbeat rate limit exceeded.
    #[error("Heartbeat rate limit: wait at least {min_interval_secs}s between heartbeats")]
    HeartbeatRateLimited {
        /// Minimum interval in seconds.
        min_interval_secs: u64,
    },
    /// Invalid node.
    #[error("Invalid node: {0}")]
    InvalidNode(String),
}

impl From<FederationError> for ToadStoolError {
    fn from(e: FederationError) -> Self {
        Self::Integration(toadstool::error::IntegrationError::OperationFailed {
            service: "federation".into(),
            operation: "coordinate".into(),
            reason: e.to_string(),
        })
    }
}

// ─── CloudFederationManager ───────────────────────────────────────────────────

/// Cloud federation manager with membership, heartbeats, and capability exchange.
pub struct CloudFederationManager {
    pub(in crate::cloud::federation) topology: state::CloudFederationTopology,
    pub(in crate::cloud::federation) network: state::InterCloudNetworkManager,
    pub(in crate::cloud::federation) replication: state::CloudDataReplicationManager,
    pub(crate) config: FederationConfig,
    /// Membership: node_id -> member with heartbeat
    pub(in crate::cloud::federation) members: HashMap<String, FederationMember>,
    pub(in crate::cloud::federation) heartbeat_timeout_secs: u64,
}

impl CloudFederationManager {
    /// Creates a new cloud federation manager.
    pub async fn new(config: FederationConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            topology: state::CloudFederationTopology::new(TopologyType::default()),
            network: state::InterCloudNetworkManager::new(NetworkConfig::default()),
            replication: state::CloudDataReplicationManager::new(ReplicationConfig::default()),
            config,
            members: HashMap::new(),
            heartbeat_timeout_secs: DEFAULT_HEARTBEAT_TIMEOUT_SECS,
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
