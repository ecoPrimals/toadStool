//! Service Registry - Dynamic service discovery
//!
//! Eliminates hardcoded primal/service names, enabling:
//! - Dynamic service discovery
//! - Environment-specific service configuration
//! - Multi-instance deployments
//! - Flexible ecosystem integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Service registry error types
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Service not found: {0}")]
    NotFound(String),

    #[error("Service already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Invalid service configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

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

    /// Endpoint URL/address (e.g., "http://localhost:7777")
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
    pub fn with_port(mut self, port: u16) -> Self {
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

/// Service registry
///
/// Provides centralized service discovery and management.
///
/// # Examples
///
/// ```rust
/// use toadstool_config::services::{ServiceRegistry, ServiceEndpoint, ServiceType};
///
/// let mut registry = ServiceRegistry::default();
///
/// // Register songbird coordinator
/// let songbird = ServiceEndpoint::new("songbird", ServiceType::Coordinator, "http://localhost:7777")
///     .with_health_check("/health");
/// registry.register(songbird);
///
/// // Find coordinator
/// let coord = registry.coordinator();
/// assert!(coord.is_some());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceRegistry {
    /// Registered services by name
    #[serde(rename = "services")]
    services: HashMap<String, ServiceEndpoint>,

    /// Services indexed by type
    #[serde(skip)]
    by_type: HashMap<ServiceType, Vec<String>>,
}

impl ServiceRegistry {
    /// Create a new empty service registry
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a service
    ///
    /// # Errors
    /// Returns `ServiceError::AlreadyRegistered` if service name is already in use.
    pub fn register(&mut self, service: ServiceEndpoint) -> ServiceResult<()> {
        let name = service.name.clone();

        if self.services.contains_key(&name) {
            return Err(ServiceError::AlreadyRegistered(name));
        }

        let stype = service.service_type.clone();

        self.services.insert(name.clone(), service);
        self.by_type.entry(stype).or_default().push(name);

        Ok(())
    }

    /// Register or update a service
    pub fn register_or_update(&mut self, service: ServiceEndpoint) {
        let name = service.name.clone();
        let stype = service.service_type.clone();

        // Remove from old type index if exists
        if let Some(old_service) = self.services.get(&name) {
            if let Some(names) = self.by_type.get_mut(&old_service.service_type) {
                names.retain(|n| n != &name);
            }
        }

        self.services.insert(name.clone(), service);
        self.by_type.entry(stype).or_default().push(name);
    }

    /// Get a service by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ServiceEndpoint> {
        self.services.get(name)
    }

    /// Find services by type
    #[must_use]
    pub fn find_by_type(&self, stype: &ServiceType) -> Vec<&ServiceEndpoint> {
        self.by_type
            .get(stype)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.services.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get coordinator service (replaces hardcoded "songbird")
    ///
    /// Returns the first registered coordinator service.
    #[must_use]
    pub fn coordinator(&self) -> Option<&ServiceEndpoint> {
        self.find_by_type(&ServiceType::Coordinator)
            .first()
            .copied()
    }

    /// Get storage service (replaces hardcoded "squirrel")
    ///
    /// Returns the first registered storage service.
    #[must_use]
    pub fn storage(&self) -> Option<&ServiceEndpoint> {
        self.find_by_type(&ServiceType::Storage).first().copied()
    }

    /// Get compute service
    #[must_use]
    pub fn compute(&self) -> Option<&ServiceEndpoint> {
        self.find_by_type(&ServiceType::Compute).first().copied()
    }

    /// Get all registered services
    #[must_use]
    pub fn all_services(&self) -> Vec<&ServiceEndpoint> {
        self.services.values().collect()
    }

    /// Get service count
    #[must_use]
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Check if registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Create registry from environment variables
    ///
    /// Reads services from environment variables:
    /// - `TOADSTOOL_COORDINATOR` - Coordinator endpoint (format: "name:endpoint")
    /// - `TOADSTOOL_STORAGE` - Storage endpoint
    /// - `TOADSTOOL_SERVICES` - JSON array of service endpoints
    #[must_use]
    pub fn from_env() -> Self {
        let mut registry = Self::default();

        // Load coordinator (e.g., songbird)
        if let Ok(coord_str) = std::env::var("TOADSTOOL_COORDINATOR") {
            if let Some((name, endpoint)) = coord_str.split_once(':') {
                let service =
                    ServiceEndpoint::new(name.trim(), ServiceType::Coordinator, endpoint.trim());
                registry.register_or_update(service);
            }
        }

        // Load storage (e.g., squirrel)
        if let Ok(storage_str) = std::env::var("TOADSTOOL_STORAGE") {
            if let Some((name, endpoint)) = storage_str.split_once(':') {
                let service =
                    ServiceEndpoint::new(name.trim(), ServiceType::Storage, endpoint.trim());
                registry.register_or_update(service);
            }
        }

        // Load services from JSON
        if let Ok(services_json) = std::env::var("TOADSTOOL_SERVICES") {
            if let Ok(services) = serde_json::from_str::<Vec<ServiceEndpoint>>(&services_json) {
                for service in services {
                    registry.register_or_update(service);
                }
            }
        }

        registry
    }

    /// Load registry from TOML file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed.
    pub fn from_toml_file(path: impl AsRef<Path>) -> ServiceResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        let registry: Self = toml::from_str(&contents)
            .map_err(|e| ServiceError::Parse(format!("TOML parse error: {}", e)))?;
        Ok(registry)
    }

    /// Load registry from JSON file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed.
    pub fn from_json_file(path: impl AsRef<Path>) -> ServiceResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        let registry: Self = serde_json::from_str(&contents)
            .map_err(|e| ServiceError::Parse(format!("JSON parse error: {}", e)))?;
        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_parsing() {
        assert_eq!(
            ServiceType::parse_type("coordinator"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("compute"), ServiceType::Compute);

        match ServiceType::parse_type("custom") {
            ServiceType::Custom(s) => assert_eq!(s, "custom"),
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_service_endpoint_builder() {
        let endpoint = ServiceEndpoint::new("test", ServiceType::Compute, "http://localhost:8080")
            .with_port(8080)
            .with_capability("wasm")
            .with_health_check("/health");

        assert_eq!(endpoint.name, "test");
        assert_eq!(endpoint.port, Some(8080));
        assert_eq!(endpoint.capabilities, vec!["wasm"]);
        assert_eq!(endpoint.health_check, Some("/health".to_string()));
    }

    #[test]
    fn test_service_registry_register() {
        let mut registry = ServiceRegistry::new();

        let songbird = ServiceEndpoint::new(
            "songbird",
            ServiceType::Coordinator,
            "http://localhost:7777",
        );
        registry.register(songbird).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get("songbird").is_some());
    }

    #[test]
    fn test_service_registry_find_by_type() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "songbird",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "squirrel",
                ServiceType::Storage,
                "http://localhost:8888",
            ))
            .unwrap();

        let coordinators = registry.find_by_type(&ServiceType::Coordinator);
        assert_eq!(coordinators.len(), 1);
        assert_eq!(coordinators[0].name, "songbird");

        let storage = registry.find_by_type(&ServiceType::Storage);
        assert_eq!(storage.len(), 1);
        assert_eq!(storage[0].name, "squirrel");
    }

    #[test]
    fn test_service_registry_coordinator() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "songbird",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();

        let coord = registry.coordinator();
        assert!(coord.is_some());
        assert_eq!(coord.unwrap().name, "songbird");
    }

    #[test]
    fn test_service_registry_storage() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "squirrel",
                ServiceType::Storage,
                "http://localhost:8888",
            ))
            .unwrap();

        let storage = registry.storage();
        assert!(storage.is_some());
        assert_eq!(storage.unwrap().name, "squirrel");
    }

    #[test]
    fn test_service_registry_already_registered() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "test",
                ServiceType::Compute,
                "http://localhost:8080",
            ))
            .unwrap();

        let result = registry.register(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:9090",
        ));
        assert!(matches!(result, Err(ServiceError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_service_registry_register_or_update() {
        let mut registry = ServiceRegistry::new();

        registry.register_or_update(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:8080",
        ));
        assert_eq!(
            registry.get("test").unwrap().endpoint,
            "http://localhost:8080"
        );

        registry.register_or_update(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:9090",
        ));
        assert_eq!(
            registry.get("test").unwrap().endpoint,
            "http://localhost:9090"
        );
    }
}
