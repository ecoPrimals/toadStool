// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service type definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Service registry error types.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Service not found in registry.
    #[error("Service not found: {0}")]
    NotFound(String),

    /// Service name already registered.
    #[error("Service already registered: {0}")]
    AlreadyRegistered(String),

    /// Invalid service configuration.
    #[error("Invalid service configuration: {0}")]
    InvalidConfig(String),

    /// File I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error (TOML, JSON, etc.).
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Service registry result type
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Service type in the ecosystem
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    /// Coordination service (e.g., songbird)
    Coordinator,

    /// Storage service (e.g., squirrel)
    Storage,

    /// Compute service (e.g., toadstool)
    Compute,

    /// Messaging/queue service
    Messaging,

    /// Database service
    Database,

    /// Cache service
    Cache,

    /// Monitoring service
    Monitoring,

    /// Custom service type
    Custom(String),
}

impl ServiceType {
    /// Parse service type from string
    #[must_use]
    pub fn parse_type(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "coordinator" => Self::Coordinator,
            "storage" => Self::Storage,
            "compute" => Self::Compute,
            "messaging" => Self::Messaging,
            "database" => Self::Database,
            "cache" => Self::Cache,
            "monitoring" => Self::Monitoring,
            _ => Self::Custom(s.to_string()),
        }
    }

    /// Get string representation
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Coordinator => "coordinator",
            Self::Storage => "storage",
            Self::Compute => "compute",
            Self::Messaging => "messaging",
            Self::Database => "database",
            Self::Cache => "cache",
            Self::Monitoring => "monitoring",
            Self::Custom(s) => s,
        }
    }
}

/// Service endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name (e.g., "songbird", "squirrel", "toadstool")
    pub name: String,

    /// Service type
    #[serde(rename = "type")]
    pub service_type: ServiceType,

    /// Endpoint URL/address (e.g., "<http://localhost:7777>")
    pub endpoint: String,

    /// Port (optional, extracted from endpoint if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Capabilities advertised by this service
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Health check endpoint (relative to endpoint)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<String>,

    /// Service metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ServiceEndpoint {
    /// Create a new service endpoint
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        service_type: ServiceType,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            service_type,
            endpoint: endpoint.into(),
            port: None,
            capabilities: Vec::new(),
            health_check: None,
            metadata: HashMap::new(),
        }
    }

    /// Set port
    #[must_use]
    pub const fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Add capability
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Add capabilities
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Set health check endpoint
    #[must_use]
    pub fn with_health_check(mut self, health_check: impl Into<String>) -> Self {
        self.health_check = Some(health_check.into());
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
