//! Service discovery types

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::primal_identity::{Capability, PrimalIdentity, ServiceEndpoint};

/// Service discovery error types
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("No services found with capability: {capability:?}")]
    NoServiceFound { capability: Capability },

    #[error("Discovery timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Discovery method unavailable: {method}")]
    MethodUnavailable { method: String },

    #[error("Invalid service response: {reason}")]
    InvalidResponse { reason: String },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        source: std::io::Error,
    },
}

pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Discovered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub endpoints: Vec<ServiceEndpoint>,
    pub metadata: HashMap<String, String>,
    pub discovered_at: SystemTime,
    pub last_seen: SystemTime,
    pub healthy: bool,
}

impl DiscoveredService {
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    #[must_use]
    pub fn primary_endpoint(&self) -> Option<&ServiceEndpoint> {
        self.endpoints.first()
    }

    #[must_use]
    pub fn healthy_endpoints(&self) -> Vec<&ServiceEndpoint> {
        self.endpoints.iter().collect()
    }

    #[must_use]
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.last_seen
            .elapsed()
            .map(|elapsed| elapsed < ttl)
            .unwrap_or(false)
    }
}

/// Service discovery trait
// TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
#[async_trait]
pub trait ServiceDiscoveryTrait: Send + Sync {
    async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> DiscoveryResult<Vec<DiscoveredService>>;
    async fn discover_all(&self) -> DiscoveryResult<Vec<DiscoveredService>>;
    async fn announce_self(&self, identity: &dyn PrimalIdentity) -> DiscoveryResult<()>;
    async fn refresh(&self) -> DiscoveryResult<()>;
}

/// Discovery method types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryMethod {
    Auto,
    Mdns,
    Environment,
    ConfigFile { path: String },
    Registry { endpoint: String },
    Multi(Vec<Self>),
}
