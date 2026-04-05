// SPDX-License-Identifier: AGPL-3.0-or-later

//! # Orchestration Network Configuration — Type Definitions
//!
//! Capability-based network configuration types. These are agnostic to which
//! primal provides orchestration — discovered at runtime.
//!
//! Many types use base configurations from `toadstool_common::config_bases`
//! for consistency and code reuse.

mod dns_discovery;
mod load_balancing;
mod network_policies;
mod reliability;
mod security;
mod service_mesh;
mod traffic;

pub use dns_discovery::*;
pub use load_balancing::*;
pub use network_policies::*;
pub use reliability::*;
pub use security::*;
pub use service_mesh::*;
pub use traffic::*;

use serde::{Deserialize, Serialize};

/// Orchestration network configuration for the coordination / service-mesh stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationNetworkConfig {
    /// Service mesh configuration
    pub service_mesh: ServiceMeshConfig,
    /// DNS service discovery configuration
    pub dns_discovery: DnsDiscoveryConfig,
    /// Cross-primal security configuration
    pub cross_primal_security: CrossPrimalSecurityConfig,
    /// Network ingress/egress rules
    pub network_policies: NetworkPoliciesConfig,
    /// Traffic management configuration
    pub traffic_management: TrafficManagementConfig,
    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Health monitoring configuration
    pub health_monitoring: HealthMonitoringConfig,
}

/// Legacy alias — prefer [`OrchestrationNetworkConfig`].
pub type SongbirdNetworkConfig = OrchestrationNetworkConfig;
