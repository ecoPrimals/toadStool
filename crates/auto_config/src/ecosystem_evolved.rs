//! # Ecosystem Discovery for Auto-Configuration - EVOLVED
//!
//! **DEEP DEBT EVOLUTION**: Capability-based discovery using IPC helpers
//!
//! Discovers available ecosystem services via:
//! - Unix socket discovery (IPC helpers)
//! - Songbird service registry
//! - Environment variables
//! - NO PORT SCANNING - capability-based only!

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::{ToadStoolError, ToadStoolResult};

/// Ecosystem discovery system using capability-based IPC
///
/// **EVOLVED**: No port scanning, only runtime discovery via Songbird
pub struct EcosystemDiscoverer {
    /// Discovered services (cached)
    discovered_services: HashMap<String, DiscoveredService>,
    /// Discovery timeout
    discovery_timeout: Duration,
}

impl EcosystemDiscoverer {
    /// Create a new ecosystem discoverer
    #[must_use]
    pub fn new() -> Self {
        Self {
            discovered_services: HashMap::new(),
            discovery_timeout: Duration::from_secs(5),
        }
    }

    /// Discover services via Songbird registry
    ///
    /// **EVOLVED**: Uses IPC helpers to query Songbird for service registry
    pub async fn discover_via_songbird(&mut self) -> ToadStoolResult<Vec<DiscoveredService>> {
        info!("🌍 Discovering services via Songbird registry");

        // Use IPC helper to resolve Songbird
        let songbird_socket = std::env::var("SONGBIRD_SOCKET")
            .unwrap_or_else(|_| "/primal/songbird".to_string());

        // Query Songbird for service list
        match self.query_songbird_registry(&songbird_socket).await {
            Ok(services) => {
                info!("✅ Discovered {} services via Songbird", services.len());
                for service in &services {
                    self.discovered_services
                        .insert(service.name.clone(), service.clone());
                }
                Ok(services)
            }
            Err(e) => {
                warn!("⚠️ Songbird discovery failed: {}", e);
                warn!("💡 Falling back to environment-based discovery");
                self.discover_from_environment()
            }
        }
    }

    /// Query Songbird registry via Unix socket
    async fn query_songbird_registry(
        &self,
        _socket_path: &str,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        // TODO: Implement JSON-RPC call to Songbird
        // Method: "ipc.list"
        // Returns list of registered services
        
        // For now, return empty list (will be implemented when Songbird integration is complete)
        debug!("🚧 Songbird registry query not yet implemented");
        Err(ToadStoolError::not_found(
            "Songbird registry query not yet implemented",
        ))
    }

    /// Discover services from environment variables
    ///
    /// **EVOLVED**: Only checks Unix socket paths, no HTTP endpoints
    fn discover_from_environment(&mut self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Check for known primal socket paths
        let known_primals = vec![
            ("beardog", "/primal/beardog", vec!["crypto", "security"]),
            ("songbird", "/primal/songbird", vec!["coordination", "network"]),
            ("nestgate", "/primal/nestgate", vec!["storage", "data"]),
            ("squirrel", "/primal/squirrel", vec!["ai", "mcp"]),
        ];

        for (name, default_socket, capabilities) in known_primals {
            // Check for environment override
            let env_var = format!("{}_SOCKET", name.to_uppercase());
            let socket_path = std::env::var(&env_var).unwrap_or_else(|_| default_socket.to_string());

            // Check if socket exists
            if std::path::Path::new(&socket_path).exists() {
                debug!("✅ Found {} at {}", name, socket_path);
                let service = DiscoveredService {
                    name: name.to_string(),
                    socket_path: Some(socket_path),
                    capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
                    available: true,
                };
                self.discovered_services.insert(name.to_string(), service.clone());
                services.push(service);
            } else {
                debug!("⚠️ Socket not found for {}: {}", name, socket_path);
            }
        }

        if services.is_empty() {
            warn!("⚠️ No services discovered from environment");
        } else {
            info!("✅ Discovered {} services from environment", services.len());
        }

        Ok(services)
    }

    /// Get discovered service by name
    pub fn get_service(&self, name: &str) -> Option<&DiscoveredService> {
        self.discovered_services.get(name)
    }

    /// Get discovered service by capability
    pub fn get_service_by_capability(&self, capability: &str) -> Option<&DiscoveredService> {
        self.discovered_services
            .values()
            .find(|s| s.capabilities.contains(&capability.to_string()))
    }

    /// List all discovered services
    pub fn list_services(&self) -> Vec<&DiscoveredService> {
        self.discovered_services.values().collect()
    }
}

impl Default for EcosystemDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovered service information
///
/// **EVOLVED**: Uses Unix sockets, not HTTP endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Service name (e.g., "beardog", "songbird")
    pub name: String,
    /// Unix socket path
    pub socket_path: Option<String>,
    /// Service capabilities
    pub capabilities: Vec<String>,
    /// Service availability
    pub available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discoverer_creation() {
        let discoverer = EcosystemDiscoverer::new();
        assert_eq!(discoverer.discovered_services.len(), 0);
    }

    #[tokio::test]
    async fn test_environment_discovery() {
        let mut discoverer = EcosystemDiscoverer::new();
        
        // This will discover services if sockets exist
        let result = discoverer.discover_from_environment();
        
        // Should not fail (may find 0 services)
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_service_by_capability() {
        let mut discoverer = EcosystemDiscoverer::new();
        
        // Add a mock service
        let service = DiscoveredService {
            name: "test_service".to_string(),
            socket_path: Some("/primal/test".to_string()),
            capabilities: vec!["crypto".to_string()],
            available: true,
        };
        
        discoverer.discovered_services.insert("test_service".to_string(), service);
        
        // Should find by capability
        let found = discoverer.get_service_by_capability("crypto");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_service");
    }
}
