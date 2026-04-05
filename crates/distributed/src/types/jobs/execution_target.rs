// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution placement targets and load-balancing strategies.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::resources::ResourceConstraints;

/// Execution target for job placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTarget {
    /// Execute locally
    Local,
    /// Execute on specific `ToadStool` instance
    ToadStool {
        /// Instance ID.
        instance_id: String,
        /// Instance endpoint URL.
        endpoint: String,
    },
    /// Execute on ecosystem service
    EcosystemService {
        /// Service name.
        service_name: String,
        /// Service endpoint URL.
        endpoint: String,
    },
    /// Execute on best available resource
    BestAvailable {
        /// Placement constraints.
        constraints: ResourceConstraints,
    },
    /// Execute with load balancing
    LoadBalanced {
        /// Load balancing strategy.
        strategy: LoadBalancingStrategy,
    },
}

/// Load balancing strategies for distributed job routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Rotate through nodes in order.
    RoundRobin,
    /// Prefer node with fewest active connections.
    LeastConnections,
    /// Round-robin with per-node weights.
    WeightedRoundRobin {
        /// Node ID to weight mapping.
        weights: HashMap<String, u32>,
    },
    /// Select based on CPU/memory availability.
    ResourceAware,
    /// Select based on observed latency.
    LatencyBased,
}
