// SPDX-License-Identifier: AGPL-3.0-only
//! Primal Identity System - Self-Knowledge Only
//!
//! This module implements the principle that each primal only knows about itself.
//! Other primals are discovered at runtime through capability-based discovery.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge**: Each primal knows only its own identity
//! - **Runtime Discovery**: Other primals discovered via capabilities
//! - **Zero Hardcoding**: No primal-specific code or configuration
//! - **Capability-Based**: Services matched by what they can do, not who they are

use std::collections::HashMap;

use crate::constants::PRIMAL_NAME;

pub use types::*;

mod types;

/// Primal identity trait - defines what a primal knows about itself
pub trait PrimalIdentity: Send + Sync {
    /// Get the primal's name (e.g., "toadstool", "songbird")
    fn primal_name(&self) -> &'static str;

    /// Get the primal's version
    fn version(&self) -> &str;

    /// Get the primal's capabilities
    fn capabilities(&self) -> Vec<Capability>;

    /// Get the primal's endpoints
    fn endpoints(&self) -> Vec<ServiceEndpoint>;

    /// Get additional metadata
    fn metadata(&self) -> HashMap<String, String>;
}

/// ToadStool's self-knowledge implementation
#[derive(Debug, Clone)]
pub struct ToadStoolIdentity {
    /// Version from Cargo.toml
    version: String,

    /// Capabilities we provide
    capabilities: Vec<Capability>,

    /// Our service endpoints
    endpoints: Vec<ServiceEndpoint>,

    /// Additional metadata
    metadata: HashMap<String, String>,
}

impl ToadStoolIdentity {
    /// Create a new ToadStool identity
    #[must_use]
    pub fn new() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert(
            "description".to_string(),
            "Universal Compute Platform".to_string(),
        );
        metadata.insert("platform".to_string(), std::env::consts::OS.to_string());
        metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: Self::default_capabilities(),
            endpoints: Vec::new(),
            metadata,
        }
    }

    /// Get default capabilities for ToadStool
    fn default_capabilities() -> Vec<Capability> {
        vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Compute(ComputeCapability::ContainerOrchestration),
            Capability::Compute(ComputeCapability::WasmExecution),
            Capability::Compute(ComputeCapability::PythonExecution),
            Capability::Compute(ComputeCapability::GpuCompute),
        ]
    }

    /// Add an endpoint
    pub fn add_endpoint(&mut self, endpoint: ServiceEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Set endpoints from configuration
    #[must_use]
    pub fn with_endpoints(mut self, endpoints: Vec<ServiceEndpoint>) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Add a custom capability
    pub fn add_capability(&mut self, capability: Capability) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl Default for ToadStoolIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimalIdentity for ToadStoolIdentity {
    fn primal_name(&self) -> &'static str {
        PRIMAL_NAME
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    fn endpoints(&self) -> Vec<ServiceEndpoint> {
        self.endpoints.clone()
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }
}

/// Discovered service information (what we learn about others)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredService {
    /// Service ID (if available)
    pub id: Option<String>,

    /// Capabilities this service provides
    pub capabilities: Vec<Capability>,

    /// Endpoints to reach this service
    pub endpoints: Vec<ServiceEndpoint>,

    /// Health status
    pub healthy: bool,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl DiscoveredService {
    /// Check if this service has a specific capability
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if this service has any compute capability
    #[must_use]
    pub fn has_compute_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Compute(_)))
    }

    /// Check if this service has any storage capability
    #[must_use]
    pub fn has_storage_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Storage(_)))
    }

    /// Check if this service has any auth capability
    #[must_use]
    pub fn has_auth_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Authentication(_)))
    }

    /// Get endpoints for a specific protocol
    #[must_use]
    pub fn endpoints_for_protocol(&self, protocol: &str) -> Vec<&ServiceEndpoint> {
        self.endpoints
            .iter()
            .filter(|ep| ep.protocol == protocol)
            .collect()
    }
}

#[cfg(test)]
mod tests;
