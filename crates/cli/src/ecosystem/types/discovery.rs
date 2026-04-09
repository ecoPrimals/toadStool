// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery scan results, discovered services, and capability-oriented service type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::time::Duration;

use super::endpoint::{ServiceEndpoint, TrustLevel};

/// Result of a service discovery scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Discovered service endpoints
    pub services: Vec<ServiceEndpoint>,
    /// How long the scan took
    pub scan_duration: Duration,
    /// Total number of services found
    pub total_discovered: usize,
    /// Number cryptographically verified
    pub verified_count: usize,
}

/// Service discovered during a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(deprecated)] // Using ServiceType during migration period
pub struct DiscoveredService {
    /// Capability type (discovery, crypto, storage, compute)
    pub service_type: ServiceType,
    /// Network address
    pub address: SocketAddr,
    /// How the service was discovered
    pub trust_level: TrustLevel,
    /// Capability key-value pairs
    pub capabilities: HashMap<String, String>,
    /// When the service was last seen
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_seen: std::time::SystemTime,
}

/// Service capability type (deprecated: use capability-based discovery)
#[deprecated(
    since = "0.2.0",
    note = "Use capability-based service identification. See service_type.rs"
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceType {
    /// Discovery/coordination capability
    Discovery,
    /// Crypto/security capability
    Crypto,
    /// Storage capability
    Storage,
    /// Compute capability (self-identity)
    Compute,
    /// Generic service identified by capabilities
    Generic,
}

#[expect(deprecated)] // ServiceType impl; deprecated during migration to capability-based discovery
impl ServiceType {
    /// Map to capability name
    pub const fn to_capability(&self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Crypto => "crypto",
            Self::Storage => "storage",
            Self::Compute => "compute",
            Self::Generic => "generic",
        }
    }

    /// Create from capability (capability-based discovery)
    pub fn from_capability(capability: &str) -> Self {
        match capability {
            "discovery" | "orchestration" | "coordination" => Self::Discovery,
            "crypto" | "pki" | "security" => Self::Crypto,
            "storage" => Self::Storage,
            "compute" | "compute:execution" | "intelligence" | "routing" => Self::Compute,
            _ => Self::Generic,
        }
    }

    /// Create from service name (backward compatibility when parsing discovered services).
    /// Resolves legacy orchestrator labels via [`toadstool_common::interned_strings::CapabilityDomain::from_label`].
    pub fn from_name(name: &str) -> Self {
        use toadstool_common::interned_strings::CapabilityDomain;
        match CapabilityDomain::from_label(name) {
            Some(domain) => Self::from_capability(domain.as_str()),
            None => Self::from_capability(name),
        }
    }
}
