// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service discovery types

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::primal_identity::{Capability, PrimalIdentity, ServiceEndpoint};

/// Service discovery error types
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// No services advertise the requested capability.
    #[error("No services found with capability: {capability:?}")]
    NoServiceFound {
        /// The capability that was queried.
        capability: Capability,
    },

    /// Discovery did not complete within the allowed time.
    #[error("Discovery timeout after {duration:?}")]
    Timeout {
        /// How long discovery was allowed to run.
        duration: Duration,
    },

    /// The chosen discovery backend is not available.
    #[error("Discovery method unavailable: {method}")]
    MethodUnavailable {
        /// Name of the unavailable method.
        method: String,
    },

    /// A discovered service returned invalid or malformed data.
    #[error("Invalid service response: {reason}")]
    InvalidResponse {
        /// Human-readable description of the invalidity.
        reason: String,
    },

    /// Discovery configuration is invalid or incomplete.
    #[error("Configuration error: {reason}")]
    ConfigError {
        /// Human-readable description of the configuration problem.
        reason: String,
    },

    /// A network I/O error occurred during discovery.
    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

/// Result type for discovery operations.
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Discovered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Unique identifier of the service instance.
    pub id: String,
    /// Human-readable service name.
    pub name: String,
    /// Service version string.
    pub version: String,
    /// Capabilities this service advertises.
    pub capabilities: Vec<Capability>,
    /// Network endpoints where the service can be reached.
    pub endpoints: Vec<ServiceEndpoint>,
    /// Arbitrary key-value metadata from discovery.
    pub metadata: HashMap<String, String>,
    /// When this service was first discovered.
    pub discovered_at: SystemTime,
    /// When this service was last seen or refreshed.
    pub last_seen: SystemTime,
    /// Whether the service is currently considered healthy.
    pub healthy: bool,
}

impl DiscoveredService {
    /// Construct a newly-discovered service with `discovered_at` and `last_seen` set to now.
    ///
    /// `metadata` defaults to empty; callers can populate it after construction.
    #[must_use]
    pub fn discovered_now(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        capabilities: Vec<Capability>,
        endpoints: Vec<ServiceEndpoint>,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            capabilities,
            endpoints,
            metadata: HashMap::new(),
            discovered_at: now,
            last_seen: now,
            healthy: true,
        }
    }

    /// Attach a single key-value metadata pair (builder-style).
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Returns whether this service advertises the given capability.
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Returns the first (primary) endpoint, if any.
    #[must_use]
    pub fn primary_endpoint(&self) -> Option<&ServiceEndpoint> {
        self.endpoints.first()
    }

    /// Returns all endpoints for this service.
    #[must_use]
    pub fn healthy_endpoints(&self) -> Vec<&ServiceEndpoint> {
        self.endpoints.iter().collect()
    }

    /// Returns whether `last_seen` is within the given TTL.
    #[must_use]
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.last_seen
            .elapsed()
            .map(|elapsed| elapsed < ttl)
            .unwrap_or(false)
    }
}

/// Service discovery trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait ServiceDiscoveryTrait: Send + Sync {
    /// Finds all services that advertise the given capability.
    async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> DiscoveryResult<Vec<DiscoveredService>>;
    /// Discovers all known services.
    async fn discover_all(&self) -> DiscoveryResult<Vec<DiscoveredService>>;
    /// Registers this primal's identity with the discovery backend.
    async fn announce_self(&self, identity: &dyn PrimalIdentity) -> DiscoveryResult<()>;
    /// Refreshes the discovery cache or re-queries the backend.
    async fn refresh(&self) -> DiscoveryResult<()>;
}

/// Discovery method types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Automatically select the best available method.
    Auto,
    /// Use mDNS/Bonjour for local network discovery.
    Mdns,
    /// Read service list from environment variables.
    Environment,
    /// Load service list from a configuration file.
    ConfigFile {
        /// Path to the config file.
        path: String,
    },
    /// Query a central registry service.
    Registry {
        /// Registry service endpoint URL.
        endpoint: String,
    },
    /// Try multiple methods in sequence.
    Multi(Vec<Self>),
}
