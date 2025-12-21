//! Port Registry - Centralized port management
//!
//! Eliminates hardcoded ports throughout the codebase, enabling:
//! - Dynamic port allocation
//! - Environment-specific configuration
//! - Multi-instance deployments
//! - Zero port conflicts in testing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::RwLock;

/// Port registry error types
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Port {0} is already in use")]
    AlreadyInUse(u16),

    #[error("No available ports in range {0}-{1}")]
    NoAvailablePorts(u16, u16),

    #[error("Invalid port number: {0}")]
    InvalidPort(String),

    #[error("Port allocation failed: {0}")]
    AllocationFailed(String),
}

/// Port registry result type
pub type PortResult<T> = Result<T, PortError>;

/// Port registry for all ToadStool services
///
/// Provides centralized port management with environment variable overrides,
/// dynamic allocation, and conflict prevention.
///
/// # Examples
///
/// ```rust
/// use toadstool_config::ports::PortRegistry;
///
/// let registry = PortRegistry::default();
///
/// // Get default API port
/// let api_port = registry.api_server();
/// assert_eq!(api_port, 8080);
///
/// // Allocate dynamic port (range 10000-20000)
/// let dynamic = registry.allocate_dynamic().unwrap();
/// assert!(dynamic >= 10000 && dynamic <= 20000);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRegistry {
    /// Main API server port (default: 8080)
    api_server: u16,

    /// WebSocket server port (default: 8081)
    websocket: u16,

    /// Metrics/monitoring port (default: 9090)
    metrics: u16,

    /// Health check port (default: 8082)
    health: u16,

    /// Container default port (default: 8080)
    container_default: u16,

    /// Edge discovery ports (default: common service ports)
    edge_discovery: Vec<u16>,

    /// Custom service ports (dynamically allocated)
    #[serde(skip)]
    custom_services: Arc<RwLock<HashMap<String, u16>>>,

    /// Dynamic port allocation range (default: 10000-20000)
    dynamic_port_range: (u16, u16),

    /// Next available dynamic port
    #[serde(skip)]
    next_dynamic_port: Arc<RwLock<u16>>,
}

impl Default for PortRegistry {
    fn default() -> Self {
        Self {
            api_server: 8080,
            websocket: 8081,
            metrics: 9090,
            health: 8082,
            container_default: 8080,
            edge_discovery: vec![22, 80, 443, 8080, 8443, 3000, 5000],
            custom_services: Arc::new(RwLock::new(HashMap::new())),
            dynamic_port_range: (10000, 20000),
            next_dynamic_port: Arc::new(RwLock::new(10000)),
        }
    }
}

impl PortRegistry {
    /// Create a new port registry with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create port registry from environment variables
    ///
    /// Reads ports from environment variables:
    /// - `TOADSTOOL_API_PORT` - API server port
    /// - `TOADSTOOL_WEBSOCKET_PORT` - WebSocket port
    /// - `TOADSTOOL_METRICS_PORT` - Metrics port
    /// - `TOADSTOOL_HEALTH_PORT` - Health check port
    /// - `TOADSTOOL_CONTAINER_PORT` - Container default port
    /// - `TOADSTOOL_EDGE_DISCOVERY_PORTS` - Comma-separated port list
    #[must_use]
    pub fn from_env() -> Self {
        let mut registry = Self::default();

        // API server port
        if let Ok(port_str) = std::env::var("TOADSTOOL_API_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                registry.api_server = port;
            }
        }

        // WebSocket port
        if let Ok(port_str) = std::env::var("TOADSTOOL_WEBSOCKET_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                registry.websocket = port;
            }
        }

        // Metrics port
        if let Ok(port_str) = std::env::var("TOADSTOOL_METRICS_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                registry.metrics = port;
            }
        }

        // Health check port
        if let Ok(port_str) = std::env::var("TOADSTOOL_HEALTH_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                registry.health = port;
            }
        }

        // Container default port
        if let Ok(port_str) = std::env::var("TOADSTOOL_CONTAINER_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                registry.container_default = port;
            }
        }

        // Edge discovery ports (comma-separated)
        if let Ok(ports_str) = std::env::var("TOADSTOOL_EDGE_DISCOVERY_PORTS") {
            let ports: Vec<u16> = ports_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if !ports.is_empty() {
                registry.edge_discovery = ports;
            }
        }

        registry
    }

    // ============================================================================
    // Port Getters
    // ============================================================================

    /// Get API server port
    #[must_use]
    pub fn api_server(&self) -> u16 {
        self.api_server
    }

    /// Get WebSocket server port
    #[must_use]
    pub fn websocket(&self) -> u16 {
        self.websocket
    }

    /// Get metrics port
    #[must_use]
    pub fn metrics(&self) -> u16 {
        self.metrics
    }

    /// Get health check port
    #[must_use]
    pub fn health(&self) -> u16 {
        self.health
    }

    /// Get container default port
    #[must_use]
    pub fn container_default(&self) -> u16 {
        self.container_default
    }

    /// Get edge discovery ports
    #[must_use]
    pub fn edge_discovery_ports(&self) -> &[u16] {
        &self.edge_discovery
    }

    // ============================================================================
    // Port Setters (Builder Pattern)
    // ============================================================================

    /// Set API server port
    pub fn with_api_port(mut self, port: u16) -> Self {
        self.api_server = port;
        self
    }

    /// Set WebSocket port
    pub fn with_websocket_port(mut self, port: u16) -> Self {
        self.websocket = port;
        self
    }

    /// Set metrics port
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics = port;
        self
    }

    /// Set health check port
    pub fn with_health_port(mut self, port: u16) -> Self {
        self.health = port;
        self
    }

    /// Set container default port
    pub fn with_container_port(mut self, port: u16) -> Self {
        self.container_default = port;
        self
    }

    /// Set edge discovery ports
    pub fn with_edge_discovery_ports(mut self, ports: Vec<u16>) -> Self {
        self.edge_discovery = ports;
        self
    }

    // ============================================================================
    // Dynamic Port Allocation
    // ============================================================================

    /// Allocate a dynamic port
    ///
    /// Returns an available port in the dynamic range (10000-20000).
    /// The port is tested for availability before being returned.
    ///
    /// # Errors
    /// Returns `PortError::NoAvailablePorts` if no ports are available.
    pub fn allocate_dynamic(&self) -> PortResult<u16> {
        let (start, end) = self.dynamic_port_range;
        let mut next_port = self.next_dynamic_port.write().unwrap_or_else(|poisoned| {
            tracing::warn!("Lock poisoned, recovering");
            poisoned.into_inner()
        });

        // Try up to 1000 ports
        for _ in 0..1000 {
            let port = *next_port;

            // Increment for next allocation
            *next_port += 1;
            if *next_port > end {
                *next_port = start;
            }

            // Check if port is available
            if Self::is_port_available(port) {
                return Ok(port);
            }
        }

        Err(PortError::NoAvailablePorts(start, end))
    }

    /// Allocate a dynamic port for a named service
    ///
    /// If the service already has a port, returns the existing port.
    /// Otherwise, allocates a new port and registers it.
    ///
    /// # Errors
    /// Returns `PortError::NoAvailablePorts` if no ports are available.
    pub fn allocate_for_service(&self, service_name: &str) -> PortResult<u16> {
        // Check if service already has a port
        {
            let services = self.custom_services.read().unwrap_or_else(|poisoned| {
                tracing::warn!("Lock poisoned, recovering");
                poisoned.into_inner()
            });
            if let Some(&port) = services.get(service_name) {
                return Ok(port);
            }
        }

        // Allocate new port
        let port = self.allocate_dynamic()?;

        // Register service
        {
            let mut services = self.custom_services.write().unwrap_or_else(|poisoned| {
                tracing::warn!("Lock poisoned, recovering");
                poisoned.into_inner()
            });
            services.insert(service_name.to_string(), port);
        }

        Ok(port)
    }

    /// Get port for a registered service
    #[must_use]
    pub fn get_service_port(&self, service_name: &str) -> Option<u16> {
        self.custom_services
            .read()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("Lock poisoned, recovering");
                poisoned.into_inner()
            })
            .get(service_name)
            .copied()
    }

    /// Check if a port is available (not in use)
    fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    // ============================================================================
    // Testing Helpers
    // ============================================================================

    /// Create port registry optimized for testing
    ///
    /// Uses dynamic ports for all services to prevent conflicts.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            api_server: 0,        // OS assigns
            websocket: 0,         // OS assigns
            metrics: 0,           // OS assigns
            health: 0,            // OS assigns
            container_default: 0, // OS assigns
            edge_discovery: vec![],
            custom_services: Arc::new(RwLock::new(HashMap::new())),
            dynamic_port_range: (10000, 20000),
            next_dynamic_port: Arc::new(RwLock::new(10000)),
        }
    }

    /// Get all allocated ports (for debugging)
    #[must_use]
    pub fn all_allocated_ports(&self) -> Vec<(String, u16)> {
        let mut ports = vec![
            ("api_server".to_string(), self.api_server),
            ("websocket".to_string(), self.websocket),
            ("metrics".to_string(), self.metrics),
            ("health".to_string(), self.health),
            ("container_default".to_string(), self.container_default),
        ];

        let services = self.custom_services.read().unwrap_or_else(|poisoned| {
            tracing::warn!("Lock poisoned, recovering");
            poisoned.into_inner()
        });
        for (name, port) in services.iter() {
            ports.push((format!("service:{}", name), *port));
        }

        ports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_registry_default() {
        let registry = PortRegistry::default();

        assert_eq!(registry.api_server(), 8080);
        assert_eq!(registry.websocket(), 8081);
        assert_eq!(registry.metrics(), 9090);
        assert_eq!(registry.health(), 8082);
        assert_eq!(registry.container_default(), 8080);
        assert_eq!(
            registry.edge_discovery_ports(),
            &[22_u16, 80, 443, 8080, 8443, 3000, 5000]
        );
    }

    #[test]
    fn test_port_registry_builder() {
        let registry = PortRegistry::default()
            .with_api_port(9000)
            .with_websocket_port(9001)
            .with_metrics_port(9002);

        assert_eq!(registry.api_server(), 9000);
        assert_eq!(registry.websocket(), 9001);
        assert_eq!(registry.metrics(), 9002);
    }

    #[test]
    fn test_dynamic_port_allocation() {
        let registry = PortRegistry::default();

        let port1 = registry.allocate_dynamic().unwrap();
        let port2 = registry.allocate_dynamic().unwrap();

        assert!((10000..=20000).contains(&port1));
        assert!((10000..=20000).contains(&port2));
        // Ports should be different (usually, unless extreme port exhaustion)
    }

    #[test]
    fn test_service_port_allocation() {
        let registry = PortRegistry::default();

        let port1 = registry.allocate_for_service("test-service").unwrap();
        let port2 = registry.allocate_for_service("test-service").unwrap();

        // Same service should get same port
        assert_eq!(port1, port2);

        // Different service should get different port
        let port3 = registry.allocate_for_service("other-service").unwrap();
        assert_ne!(port1, port3);
    }

    #[test]
    fn test_get_service_port() {
        let registry = PortRegistry::default();

        // Service not registered yet
        assert_eq!(registry.get_service_port("unknown"), None);

        // Allocate port for service
        let port = registry.allocate_for_service("test-service").unwrap();

        // Should be able to retrieve it
        assert_eq!(registry.get_service_port("test-service"), Some(port));
    }

    #[test]
    fn test_for_testing() {
        let registry = PortRegistry::for_testing();

        // Testing registry uses OS-assigned ports (0)
        assert_eq!(registry.api_server(), 0);
        assert_eq!(registry.websocket(), 0);
        assert_eq!(registry.metrics(), 0);
    }

    #[test]
    fn test_all_allocated_ports() {
        let registry = PortRegistry::default();

        registry.allocate_for_service("service1").unwrap();
        registry.allocate_for_service("service2").unwrap();

        let ports = registry.all_allocated_ports();

        // Should have default ports + custom services
        assert!(ports.len() >= 7); // 5 default + 2 custom
    }
}
