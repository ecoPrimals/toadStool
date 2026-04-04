// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Service endpoint discovered through capability matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Capability this service provides
    pub capability: String,

    /// Endpoint URL (protocol-agnostic)
    pub endpoint: String,

    /// Supported protocols (http, rpc, grpc, etc.)
    pub protocols: Vec<String>,

    /// Service metadata
    pub metadata: ServiceMetadata,

    /// Discovery source (how we found it)
    pub source: DiscoverySource,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    /// Service version
    pub version: Option<String>,

    /// Service health status
    pub health: ServiceHealth,

    /// Last seen timestamp
    pub last_seen: std::time::SystemTime,

    /// Priority/preference score (0-100)
    pub priority: u8,

    /// Additional arbitrary metadata
    pub extra: std::collections::HashMap<String, String>,
}

impl Default for ServiceMetadata {
    fn default() -> Self {
        Self {
            version: None,
            health: ServiceHealth::Unknown,
            last_seen: std::time::SystemTime::now(),
            priority: 50,
            extra: std::collections::HashMap::new(),
        }
    }
}

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceHealth {
    /// Health status unknown or not yet checked
    Unknown,
    /// Service is degraded but may still accept requests
    Degraded,
    /// Service is healthy and ready
    Healthy,
}

/// Source of service discovery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Discovered via environment variable
    Environment,

    /// Discovered via mDNS/Bonjour
    MDNS,

    /// Discovered via service mesh (consul, etcd, etc.)
    ServiceMesh(
        /// Mesh identifier (e.g., "consul", "etcd")
        String,
    ),

    /// Discovered via configuration file
    ConfigFile,

    /// Using fallback default
    Fallback,

    /// Discovered via universal adapter
    UniversalAdapter,
}

/// Capability discovery preferences
#[derive(Debug, Clone)]
pub struct DiscoveryPreferences {
    /// Prefer local services
    pub prefer_local: bool,

    /// Required protocols (empty = any)
    pub required_protocols: Vec<String>,

    /// Timeout for discovery
    pub timeout: Option<Duration>,

    /// Minimum health level
    pub min_health: ServiceHealth,

    /// Preferred discovery sources (in order)
    pub preferred_sources: Vec<DiscoverySource>,
}

impl Default for DiscoveryPreferences {
    fn default() -> Self {
        Self {
            prefer_local: false,
            required_protocols: Vec::new(),
            timeout: None,
            min_health: ServiceHealth::Unknown,
            preferred_sources: Vec::new(),
        }
    }
}
