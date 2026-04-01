// SPDX-License-Identifier: AGPL-3.0-only
//! Coordination integration types - Protocol-level, vendor-agnostic
//!
//! **Design Philosophy**:
//! - Protocol-agnostic: Works with any coordination provider's API
//! - Version-agnostic: Handle API evolution gracefully
//! - Type-safe: Strong types, no stringly-typed data
//! - Zero vendor lock-in: These types work with ANY coordination service

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

/// Service registration request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Service unique identifier
    pub service_id: String,

    /// Service name (for display only)
    pub service_name: String,

    /// Service version
    pub version: String,

    /// Capabilities this service provides
    pub capabilities: Vec<String>,

    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// Time-to-live for registration (seconds)
    pub ttl_seconds: u64,
}

/// Service endpoint information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Protocol (http, https, grpc, etc.)
    pub protocol: String,

    /// Address
    pub address: SocketAddr,

    /// Optional path prefix
    pub path: Option<String>,

    /// Endpoint metadata
    pub metadata: HashMap<String, String>,
}

/// Coordination request (generic operation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Operation type
    pub operation: CoordinationOperation,

    /// Request metadata
    pub metadata: serde_json::Value,
}

/// Coordination response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Success status
    pub success: bool,

    /// Response data
    pub data: serde_json::Value,

    /// Response metadata
    pub metadata: serde_json::Value,
}

/// Coordination operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoordinationOperation {
    /// Register a service
    RegisterService {
        /// Full registration payload.
        registration: Box<ServiceRegistration>,
    },

    /// Deregister a service
    DeregisterService {
        /// Service id to remove.
        service_id: String,
    },

    /// Discover services by capability
    DiscoverServices {
        /// Capability name to search for.
        capability: String,
    },

    /// Get load balancing advice
    GetLoadBalancing {
        /// Service ids to consider.
        service_ids: Vec<String>,
    },

    /// Report health status
    ReportHealth {
        /// Reporting service id.
        service_id: String,
        /// Whether the service considers itself healthy.
        healthy: bool,
    },

    /// Subscribe to service updates
    Subscribe {
        /// Capability to watch.
        capability: String,
    },

    /// Unsubscribe from service updates
    Unsubscribe {
        /// Subscription id returned from a prior subscribe.
        subscription_id: String,
    },
}

/// Health check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    /// Service identifier
    pub service_id: String,

    /// Health status
    pub healthy: bool,

    /// Health check timestamp
    pub timestamp: u64,

    /// Health metrics
    pub metrics: HashMap<String, f64>,

    /// Status message
    pub message: Option<String>,
}

/// Load balancing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingRequest {
    /// Service capability to load balance
    pub capability: String,

    /// Requested capacity
    pub requested_capacity: Option<u64>,

    /// Load balancing strategy preference
    pub strategy: LoadBalancingStrategy,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Load balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,

    /// Least connections
    LeastConnections,

    /// Least response time
    LeastResponseTime,

    /// Random selection
    Random,

    /// Weighted round-robin
    WeightedRoundRobin {
        /// Relative weights per backend index.
        weights: Vec<u32>,
    },

    /// Consistent hashing
    ConsistentHash {
        /// Hash key for stable backend selection.
        key: String,
    },

    /// Custom strategy
    Custom(String),
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub node_id: String,

    /// Node address
    pub address: SocketAddr,

    /// Node capabilities
    pub capabilities: Vec<String>,

    /// Node status
    pub status: NodeStatus,

    /// Node metadata
    pub metadata: HashMap<String, String>,

    /// Last health check timestamp
    pub last_health_check: Option<u64>,

    /// Response time (milliseconds)
    pub response_time_ms: Option<u64>,
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is healthy and available
    Healthy,

    /// Node is degraded but still available
    Degraded,

    /// Node is unhealthy and unavailable
    Unhealthy,

    /// Node status unknown
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_registration_creation() {
        let registration = ServiceRegistration {
            service_id: "test-service".to_string(),
            service_name: "Test Service".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 60,
        };

        assert_eq!(registration.service_id, "test-service");
        assert_eq!(registration.ttl_seconds, 60);
    }

    #[test]
    fn test_load_balancing_strategy() {
        let strategy = LoadBalancingStrategy::RoundRobin;
        assert_eq!(strategy, LoadBalancingStrategy::RoundRobin);

        let weighted = LoadBalancingStrategy::WeightedRoundRobin {
            weights: vec![1, 2, 3],
        };
        assert!(matches!(
            weighted,
            LoadBalancingStrategy::WeightedRoundRobin { .. }
        ));
    }

    #[test]
    fn test_node_status() {
        assert_eq!(NodeStatus::Healthy, NodeStatus::Healthy);
        assert_ne!(NodeStatus::Healthy, NodeStatus::Degraded);
    }

    #[test]
    fn test_coordination_operation() {
        let op = CoordinationOperation::DiscoverServices {
            capability: "compute".to_string(),
        };
        assert!(matches!(op, CoordinationOperation::DiscoverServices { .. }));
    }
}
