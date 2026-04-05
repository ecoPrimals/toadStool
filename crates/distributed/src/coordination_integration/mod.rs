// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination Integration Module - Vendor-Agnostic Service Coordination
//!
//! **Design Philosophy (Infant Discovery)**:
//! - ✅ Zero hardcoding: Discovers coordination services by capability, not by name
//! - ✅ Self-knowledge: ToadStool knows it needs coordination, not which provider implements it
//! - ✅ Multi-vendor: Works with Coordination, Consul, etcd, K8s, Nomad, etc.
//! - ✅ Runtime discovery: Uses mDNS, service registries, or environment configuration
//! - ✅ Graceful degradation: Falls back to local coordination if no service available
//!
//! ## Migration from coordination
//!
//! This module replaces `coordination` with a capability-based approach:
//!
//! **Before (hardcoded)**:
//! ```ignore
//! use crate::coordination::{CoordinationConnection, CoordinationNetworkDiscovery};
//! let connection = CoordinationConnection::new(config).await?;
//! let discovery = CoordinationNetworkDiscovery::new().await?; // network service discovery
//! ```
//!
//! **After (capability-based)**:
//! ```ignore
//! use crate::coordination_integration::{CoordinationClient, CoordinationDiscovery};
//! use toadstool_common::primal_identity::{Capability, CoordinationCapability};
//!
//! let discovery = CoordinationDiscovery::new(config).await?;
//! let service = discovery
//!     .discover_by_capability(Capability::Coordination(CoordinationCapability::ServiceDiscovery))
//!     .await?;
//! let client = CoordinationClient::new(&service)?;
//! ```
//!
//! ## Supported Providers
//!
//! Any service advertising coordination capabilities will work:
//! - Coordination (ecoPrimals native)
//! - HashiCorp Consul
//! - etcd
//! - Kubernetes API server
//! - Apache ZooKeeper
//! - HashiCorp Nomad
//! - Netflix Eureka
//! - Local coordination (fallback)

pub mod client;
pub mod types;

pub use client::{CoordinationClient, CoordinationDiscovery};
pub use types::{
    CoordinationRequest, CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, NodeInfo,
    ServiceRegistration,
};

/// Coordination service discovery configuration
///
/// **Design**: No hardcoded endpoints, discover at runtime
#[derive(Debug, Clone)]
pub struct CoordinationConfig {
    /// Enable auto-discovery
    pub auto_discover: bool,

    /// Discovery timeout (milliseconds)
    pub discovery_timeout_ms: u64,

    /// Preferred service location
    pub preferred_location: ServiceLocation,

    /// Fallback to local coordination if no service available
    pub fallback_enabled: bool,

    /// Required capabilities (filter discovered services)
    pub required_capabilities: Vec<toadstool_common::primal_identity::CoordinationCapability>,

    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        use toadstool_common::primal_identity::CoordinationCapability;

        Self {
            auto_discover: true,
            discovery_timeout_ms: 5000,
            preferred_location: ServiceLocation::Any,
            fallback_enabled: true,
            required_capabilities: vec![
                CoordinationCapability::ServiceDiscovery,
                CoordinationCapability::LoadBalancing,
                CoordinationCapability::HealthChecking,
            ],
            health_check_interval_secs: 30,
        }
    }
}

/// Service location preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLocation {
    /// Prefer local service instance
    Local,
    /// Prefer network service
    Network,
    /// Any available
    Any,
}
