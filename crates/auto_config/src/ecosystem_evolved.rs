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

use toadstool_common::constants::ecosystem::well_known;

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
    ///
    /// **EVOLVED**: Real JSON-RPC call to Songbird using `ipc.list` method
    ///
    /// ## Protocol
    ///
    /// Songbird service registry uses JSON-RPC 2.0 over Unix sockets.
    /// Method: `ipc.list` returns array of registered services.
    async fn query_songbird_registry(
        &self,
        socket_path: &str,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        use std::path::Path;
        use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

        // Check if socket exists before attempting connection
        if !Path::new(socket_path).exists() {
            debug!(
                "Songbird socket not found at {} (service may not be running)",
                socket_path
            );
            return Err(ToadStoolError::not_found(format!(
                "Songbird socket not found: {}",
                socket_path
            )));
        }

        let client = UnixJsonRpcClient::new(socket_path);

        // Call ipc.list to get registered services
        // Timeout is handled by the client internally
        match tokio::time::timeout(self.discovery_timeout, async {
            client
                .call::<_, Vec<SongbirdServiceEntry>>("ipc.list", serde_json::json!({}))
                .await
        })
        .await
        {
            Ok(Ok(entries)) => {
                debug!("Songbird returned {} service entries", entries.len());
                let services = entries
                    .into_iter()
                    .map(|entry| DiscoveredService {
                        name: entry.name,
                        socket_path: Some(entry.socket_path),
                        capabilities: entry.capabilities,
                        available: entry.healthy,
                    })
                    .collect();
                Ok(services)
            }
            Ok(Err(e)) => {
                debug!("Songbird ipc.list call failed: {}", e);
                Err(e)
            }
            Err(_) => {
                debug!("Songbird query timed out after {:?}", self.discovery_timeout);
                Err(ToadStoolError::timeout(format!(
                    "Songbird query timed out after {:?}",
                    self.discovery_timeout
                )))
            }
        }
    }

    /// Discover services from environment variables
    ///
    /// **EVOLVED**: Only checks Unix socket paths, no HTTP endpoints
    #[allow(deprecated)] // Intentional: IPC addressing requires well-known names
    fn discover_from_environment(&mut self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Check for known primal socket paths (integration constants)
        let known_primals = vec![
            (
                well_known::BEARDOG,
                "/primal/beardog",
                vec!["crypto", "security"],
            ),
            (
                well_known::SONGBIRD,
                "/primal/songbird",
                vec!["coordination", "network"],
            ),
            (
                well_known::NESTGATE,
                "/primal/nestgate",
                vec!["storage", "data"],
            ),
            (
                well_known::SQUIRREL,
                "/primal/squirrel",
                vec!["ai", "mcp"],
            ),
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

/// Service entry from Songbird registry
///
/// This is the JSON structure returned by Songbird's `ipc.list` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SongbirdServiceEntry {
    /// Service name
    name: String,
    /// Unix socket path
    socket_path: String,
    /// Service capabilities
    #[serde(default)]
    capabilities: Vec<String>,
    /// Health status
    #[serde(default = "default_healthy")]
    healthy: bool,
}

/// Default for healthy field (assume healthy if not specified)
fn default_healthy() -> bool {
    true
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

        discoverer
            .discovered_services
            .insert("test_service".to_string(), service);

        // Should find by capability
        let found = discoverer.get_service_by_capability("crypto");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_service");
    }

    #[test]
    fn test_discovered_service_serialization() {
        let service = DiscoveredService {
            name: "beardog".to_string(),
            socket_path: Some("/primal/beardog".to_string()),
            capabilities: vec!["crypto".to_string(), "security".to_string()],
            available: true,
        };
        let json = serde_json::to_string(&service).expect("serialize");
        assert!(json.contains("beardog"));
        assert!(json.contains("crypto"));
    }

    #[test]
    fn test_list_services() {
        let mut discoverer = EcosystemDiscoverer::new();
        let service = DiscoveredService {
            name: "svc1".to_string(),
            socket_path: None,
            capabilities: vec![],
            available: true,
        };
        discoverer
            .discovered_services
            .insert("svc1".to_string(), service);
        let list = discoverer.list_services();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "svc1");
    }

    #[test]
    fn test_get_service_missing() {
        let discoverer = EcosystemDiscoverer::new();
        assert!(discoverer.get_service("nonexistent").is_none());
    }
}
