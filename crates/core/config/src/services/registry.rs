// SPDX-License-Identifier: AGPL-3.0-only
//! Service registry - centralized service discovery and management.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::types::{ServiceEndpoint, ServiceError, ServiceResult, ServiceType};

fn build_by_type_index(
    services: &HashMap<String, ServiceEndpoint>,
) -> HashMap<ServiceType, Vec<String>> {
    let mut by_type: HashMap<ServiceType, Vec<String>> = HashMap::new();
    for (name, ep) in services {
        by_type
            .entry(ep.service_type.clone())
            .or_default()
            .push(name.clone());
    }
    by_type
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
#[derive(Debug, Clone, Default, Serialize)]
pub struct ServiceRegistry {
    /// Registered services by name
    #[serde(rename = "services")]
    services: HashMap<String, ServiceEndpoint>,

    /// Services indexed by type
    #[serde(skip)]
    by_type: HashMap<ServiceType, Vec<String>>,
}

impl<'de> Deserialize<'de> for ServiceRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "services")]
            services: HashMap<String, ServiceEndpoint>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let by_type = build_by_type_index(&helper.services);
        Ok(Self {
            services: helper.services,
            by_type,
        })
    }
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
