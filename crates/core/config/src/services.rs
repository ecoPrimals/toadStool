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
            .map_err(|e| ServiceError::Parse(format!("TOML parse error: {e}")))?;
        Ok(registry)
    }

    /// Load registry from JSON file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed.
    pub fn from_json_file(path: impl AsRef<Path>) -> ServiceResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        let registry: Self = serde_json::from_str(&contents)
            .map_err(|e| ServiceError::Parse(format!("JSON parse error: {e}")))?;
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

    // =========================================================================
    // ServiceType - comprehensive tests
    // =========================================================================

    #[test]
    fn test_service_type_parse_all_variants() {
        assert_eq!(
            ServiceType::parse_type("coordinator"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("compute"), ServiceType::Compute);
        assert_eq!(ServiceType::parse_type("messaging"), ServiceType::Messaging);
        assert_eq!(ServiceType::parse_type("database"), ServiceType::Database);
        assert_eq!(ServiceType::parse_type("cache"), ServiceType::Cache);
        assert_eq!(
            ServiceType::parse_type("monitoring"),
            ServiceType::Monitoring
        );
    }

    #[test]
    fn test_service_type_parse_case_insensitive() {
        assert_eq!(
            ServiceType::parse_type("COORDINATOR"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("Storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("CoMpUtE"), ServiceType::Compute);
    }

    #[test]
    fn test_service_type_as_str_all_variants() {
        assert_eq!(ServiceType::Coordinator.as_str(), "coordinator");
        assert_eq!(ServiceType::Storage.as_str(), "storage");
        assert_eq!(ServiceType::Compute.as_str(), "compute");
        assert_eq!(ServiceType::Messaging.as_str(), "messaging");
        assert_eq!(ServiceType::Database.as_str(), "database");
        assert_eq!(ServiceType::Cache.as_str(), "cache");
        assert_eq!(ServiceType::Monitoring.as_str(), "monitoring");
        assert_eq!(
            ServiceType::Custom("my-service".into()).as_str(),
            "my-service"
        );
    }

    #[test]
    fn test_service_type_serialization_roundtrip() {
        let types = [
            ServiceType::Coordinator,
            ServiceType::Storage,
            ServiceType::Custom("mytype".into()),
        ];
        for st in &types {
            let json = serde_json::to_string(st).unwrap();
            let parsed: ServiceType = serde_json::from_str(&json).unwrap();
            assert_eq!(st, &parsed);
        }
    }

    // =========================================================================
    // ServiceEndpoint - comprehensive tests
    // =========================================================================

    #[test]
    fn test_service_endpoint_defaults() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000");
        assert_eq!(ep.name, "svc");
        assert_eq!(ep.service_type, ServiceType::Compute);
        assert_eq!(ep.endpoint, "http://localhost:9000");
        assert_eq!(ep.port, None);
        assert!(ep.capabilities.is_empty());
        assert_eq!(ep.health_check, None);
        assert!(ep.metadata.is_empty());
    }

    #[test]
    fn test_service_endpoint_with_metadata() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000")
            .with_metadata("version", "1.0")
            .with_metadata("region", "us-east");
        assert_eq!(ep.metadata.get("version"), Some(&"1.0".to_string()));
        assert_eq!(ep.metadata.get("region"), Some(&"us-east".to_string()));
    }

    #[test]
    fn test_service_endpoint_with_capabilities() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000")
            .with_capabilities(vec!["wasm".into(), "container".into()]);
        assert_eq!(ep.capabilities, vec!["wasm", "container"]);
    }

    #[test]
    fn test_service_endpoint_builder_chain() {
        let ep = ServiceEndpoint::new("toadstool", ServiceType::Compute, "http://127.0.0.1:8084")
            .with_port(8084)
            .with_capability("wasm")
            .with_capability("container")
            .with_health_check("/health")
            .with_metadata("env", "dev");
        assert_eq!(ep.port, Some(8084));
        assert_eq!(ep.capabilities.len(), 2);
        assert_eq!(ep.health_check.as_deref(), Some("/health"));
        assert_eq!(ep.metadata.get("env"), Some(&"dev".to_string()));
    }

    #[test]
    fn test_service_endpoint_serialization_roundtrip() {
        let ep = ServiceEndpoint::new("test", ServiceType::Coordinator, "http://localhost:7777")
            .with_port(7777)
            .with_capability("coordination")
            .with_health_check("/health")
            .with_metadata("version", "2.0");
        let json = serde_json::to_string(&ep).unwrap();
        let parsed: ServiceEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep.name, parsed.name);
        assert_eq!(ep.endpoint, parsed.endpoint);
        assert_eq!(ep.port, parsed.port);
        assert_eq!(ep.capabilities, parsed.capabilities);
        assert_eq!(ep.health_check, parsed.health_check);
        assert_eq!(ep.metadata, parsed.metadata);
    }

    // =========================================================================
    // ServiceRegistry - comprehensive tests
    // =========================================================================

    #[test]
    fn test_service_registry_default() {
        let registry = ServiceRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_service_registry_is_empty() {
        let mut registry = ServiceRegistry::new();
        assert!(registry.is_empty());
        registry
            .register(ServiceEndpoint::new(
                "a",
                ServiceType::Compute,
                "http://a:1",
            ))
            .unwrap();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_service_registry_all_services() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ServiceEndpoint::new(
                "a",
                ServiceType::Compute,
                "http://a:1",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "b",
                ServiceType::Storage,
                "http://b:2",
            ))
            .unwrap();
        let all = registry.all_services();
        assert_eq!(all.len(), 2);
        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_service_registry_compute() {
        let mut registry = ServiceRegistry::new();
        assert!(registry.compute().is_none());

        registry
            .register(ServiceEndpoint::new(
                "toadstool",
                ServiceType::Compute,
                "http://localhost:8084",
            ))
            .unwrap();
        let compute = registry.compute().unwrap();
        assert_eq!(compute.name, "toadstool");
    }

    #[test]
    fn test_service_registry_get_nonexistent() {
        let registry = ServiceRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_service_registry_find_by_type_empty() {
        let registry = ServiceRegistry::new();
        let results = registry.find_by_type(&ServiceType::Coordinator);
        assert!(results.is_empty());
    }

    #[test]
    fn test_service_registry_register_or_update_type_change() {
        let mut registry = ServiceRegistry::new();
        registry.register_or_update(ServiceEndpoint::new(
            "svc",
            ServiceType::Compute,
            "http://localhost:8080",
        ));
        assert_eq!(registry.find_by_type(&ServiceType::Compute).len(), 1);
        assert_eq!(registry.find_by_type(&ServiceType::Storage).len(), 0);

        registry.register_or_update(ServiceEndpoint::new(
            "svc",
            ServiceType::Storage,
            "http://localhost:8081",
        ));
        assert_eq!(registry.find_by_type(&ServiceType::Compute).len(), 0);
        assert_eq!(registry.find_by_type(&ServiceType::Storage).len(), 1);
        assert_eq!(
            registry.get("svc").unwrap().service_type,
            ServiceType::Storage
        );
    }

    #[test]
    fn test_service_registry_from_toml_file_missing() {
        let result = ServiceRegistry::from_toml_file("/nonexistent/path/to/services.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Io(_)));
    }

    #[test]
    fn test_service_registry_from_json_file_missing() {
        let result = ServiceRegistry::from_json_file("/nonexistent/path/to/services.json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Io(_)));
    }

    #[test]
    fn test_service_registry_from_json_file_invalid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_invalid_test.json");
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(b"{ invalid json }").unwrap();
        drop(f);

        let result = ServiceRegistry::from_json_file(&temp);
        std::fs::remove_file(&temp).ok();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Parse(_)));
    }

    #[test]
    fn test_service_registry_from_toml_file_valid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_valid_test.toml");
        let content = r#"
[services.songbird]
name = "songbird"
type = "coordinator"
endpoint = "http://localhost:7777"
port = 7777
"#;
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let result = ServiceRegistry::from_toml_file(&temp);
        std::fs::remove_file(&temp).ok();
        let registry = result.unwrap();
        assert_eq!(registry.len(), 1);
        let songbird = registry.get("songbird").unwrap();
        assert_eq!(songbird.service_type, ServiceType::Coordinator);
        assert_eq!(songbird.endpoint, "http://localhost:7777");
    }

    #[test]
    fn test_service_registry_from_json_file_valid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_valid_test.json");
        let content = r#"{"services":{"squirrel":{"name":"squirrel","type":"storage","endpoint":"http://localhost:8888","port":8888}}}"#;
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let result = ServiceRegistry::from_json_file(&temp);
        std::fs::remove_file(&temp).ok();
        let registry = result.unwrap();
        assert_eq!(registry.len(), 1);
        let squirrel = registry.get("squirrel").unwrap();
        assert_eq!(squirrel.service_type, ServiceType::Storage);
    }

    #[test]
    fn test_service_registry_serialization_roundtrip() {
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

        let json = serde_json::to_string(&registry).unwrap();
        let parsed: ServiceRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed.get("songbird").is_some());
        assert!(parsed.get("squirrel").is_some());
    }

    #[test]
    fn test_service_error_display() {
        let err = ServiceError::NotFound("svc".into());
        assert!(format!("{err}").contains("svc"));

        let err = ServiceError::AlreadyRegistered("dup".into());
        assert!(format!("{err}").contains("dup"));

        let err = ServiceError::InvalidConfig("bad".into());
        assert!(format!("{err}").contains("bad"));
    }

    #[test]
    fn test_service_registry_from_env_coordinator() {
        let coord_key = "TOADSTOOL_COORDINATOR";
        let orig = std::env::var(coord_key).ok();
        std::env::set_var(coord_key, "songbird:http://localhost:7777");

        let registry = ServiceRegistry::from_env();
        let coord = registry.coordinator();
        assert!(coord.is_some());
        assert_eq!(coord.unwrap().name, "songbird");
        assert_eq!(coord.unwrap().endpoint, "http://localhost:7777");

        if let Some(v) = orig {
            std::env::set_var(coord_key, v);
        } else {
            std::env::remove_var(coord_key);
        }
    }

    #[test]
    fn test_service_registry_from_env_storage() {
        let storage_key = "TOADSTOOL_STORAGE";
        let orig = std::env::var(storage_key).ok();
        std::env::set_var(storage_key, "squirrel:http://localhost:8888");

        let registry = ServiceRegistry::from_env();
        let storage = registry.storage();
        assert!(storage.is_some());
        assert_eq!(storage.unwrap().name, "squirrel");

        if let Some(v) = orig {
            std::env::set_var(storage_key, v);
        } else {
            std::env::remove_var(storage_key);
        }
    }

    #[test]
    fn test_service_registry_from_env_services_json() {
        let services_key = "TOADSTOOL_SERVICES";
        let orig = std::env::var(services_key).ok();
        let json = r#"[{"name":"custom","type":"cache","endpoint":"http://localhost:6379","capabilities":["redis"]}]"#;
        std::env::set_var(services_key, json);

        let registry = ServiceRegistry::from_env();
        let cache = registry.find_by_type(&ServiceType::Cache);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].name, "custom");
        assert_eq!(cache[0].endpoint, "http://localhost:6379");

        if let Some(v) = orig {
            std::env::set_var(services_key, v);
        } else {
            std::env::remove_var(services_key);
        }
    }

    #[test]
    fn test_service_registry_from_env_no_colon_ignored() {
        let coord_key = "TOADSTOOL_COORDINATOR";
        let orig = std::env::var(coord_key).ok();
        std::env::set_var(coord_key, "no-colon-here");

        let registry = ServiceRegistry::from_env();
        assert!(registry.coordinator().is_none());

        if let Some(v) = orig {
            std::env::set_var(coord_key, v);
        } else {
            std::env::remove_var(coord_key);
        }
    }
}
