// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime Port Discovery - Deep Debt Compliant
//!
//! NO hardcoded ports! This module discovers available ports at runtime,
//! respecting the Deep Debt principle of runtime configuration only.
//!
//! Philosophy: "Discover, don't assume. Runtime, not compile-time."

use std::net::{SocketAddr, TcpListener};
use std::ops::Range;

/// Result type for port discovery operations
pub type PortResult<T> = Result<T, PortError>;

/// Errors that can occur during port discovery
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Failed to bind to port {port}: {reason}")]
    BindFailed { port: u16, reason: String },

    #[error("No available ports found in range")]
    NoAvailablePorts,

    #[error("Failed to get local address: {0}")]
    AddressError(String),
}

/// Runtime port discovery - finds available ports dynamically
///
/// Deep Debt Principles:
/// - NO hardcoded ports
/// - Runtime discovery only
/// - Environment-agnostic
/// - Capability-based
#[derive(Debug, Clone)]
pub struct RuntimePortDiscovery {
    /// Preferred port range (if any)
    preferred_range: Option<Range<u16>>,
    /// Whether to bind to localhost only
    localhost_only: bool,
}

impl Default for RuntimePortDiscovery {
    fn default() -> Self {
        Self {
            // Unprivileged ports only (>1024)
            preferred_range: Some(8000..9000),
            localhost_only: true,
        }
    }
}

impl RuntimePortDiscovery {
    /// Create new port discovery with default settings
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover an available port, optionally with preference
    ///
    /// # Errors
    ///
    /// Returns error if no port in the preferred range can be bound.
    ///
    /// Deep Debt: If preferred port is unavailable, finds another.
    /// NO hardcoding, NO assumptions!
    pub fn discover_port(&self, preferred: Option<u16>) -> PortResult<u16> {
        // Try preferred port first
        if let Some(port) = preferred {
            if self.is_port_available(port) {
                return Ok(port);
            }
        }

        // Try range if specified
        if let Some(ref range) = self.preferred_range {
            for port in range.clone() {
                if self.is_port_available(port) {
                    return Ok(port);
                }
            }
        }

        // Last resort: let OS choose (port 0 = automatic)
        self.find_available_port(0)
    }

    /// Check if specific port is available
    fn is_port_available(&self, port: u16) -> bool {
        let addr = if self.localhost_only {
            format!("127.0.0.1:{port}")
        } else {
            format!("0.0.0.0:{port}")
        };

        match addr.parse::<SocketAddr>() {
            Ok(socket_addr) => TcpListener::bind(socket_addr).is_ok(),
            Err(_) => false,
        }
    }

    /// Find any available port (let OS choose if port=0)
    fn find_available_port(&self, port: u16) -> PortResult<u16> {
        let addr = if self.localhost_only {
            format!("127.0.0.1:{port}")
        } else {
            format!("0.0.0.0:{port}")
        };

        let listener = TcpListener::bind(&addr).map_err(|e| PortError::BindFailed {
            port,
            reason: e.to_string(),
        })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| PortError::AddressError(e.to_string()))?;

        Ok(local_addr.port())
    }

    /// Discover multiple ports at once
    ///
    /// # Errors
    ///
    /// Returns error if any required port cannot be found.
    pub fn discover_ports(&self, count: usize) -> PortResult<Vec<u16>> {
        let mut ports = Vec::with_capacity(count);
        for _ in 0..count {
            let port = self.discover_port(None)?;
            ports.push(port);
        }
        Ok(ports)
    }

    /// Set preferred port range
    #[must_use]
    pub const fn with_range(mut self, range: Range<u16>) -> Self {
        self.preferred_range = Some(range);
        self
    }

    /// Allow binding to all interfaces
    #[must_use]
    pub const fn all_interfaces(mut self) -> Self {
        self.localhost_only = false;
        self
    }

    /// Bind to localhost only (default)
    #[must_use]
    pub const fn localhost_only(mut self) -> Self {
        self.localhost_only = true;
        self
    }
}

/// Convenience function for quick port discovery
///
/// # Errors
///
/// Returns error if no available port can be found.
///
/// Deep Debt: NO hardcoded defaults, discovers at runtime
pub fn discover_available_port() -> PortResult<u16> {
    RuntimePortDiscovery::new().discover_port(None)
}

/// Discover port with preferred value
///
/// # Errors
///
/// Returns error if preferred and alternative ports cannot be bound.
///
/// Deep Debt: If preferred unavailable, finds alternative
pub fn discover_port_with_preference(preferred: u16) -> PortResult<u16> {
    RuntimePortDiscovery::new().discover_port(Some(preferred))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_available_port() {
        let port = discover_available_port();
        assert!(port.is_ok());
        let port = port.unwrap();
        assert!(port >= 1024); // Unprivileged
                               // port is u16, so always < 65536
    }

    #[test]
    fn test_discover_multiple_ports() {
        let discovery = RuntimePortDiscovery::new();
        let ports = discovery.discover_ports(3);
        assert!(ports.is_ok());
        let ports = ports.unwrap();
        assert_eq!(ports.len(), 3);

        // All should be in valid range
        for port in &ports {
            assert!(*port >= 1024);
        }
    }

    #[test]
    fn test_preferred_port_unavailable() {
        // Port 80 typically requires privileges
        let discovery = RuntimePortDiscovery::new();
        let port = discovery.discover_port(Some(80));

        // Should succeed by finding alternative
        assert!(port.is_ok());
        let port = port.unwrap();
        assert_ne!(port, 80); // Should NOT be 80
    }

    #[test]
    fn test_range_based_discovery() {
        let discovery = RuntimePortDiscovery::new().with_range(9000..9100);

        let port = discovery.discover_port(None);
        assert!(port.is_ok());
        let port = port.unwrap();
        assert!(port >= 9000);
        assert!(port < 9100);
    }

    #[test]
    fn test_is_port_available() {
        let discovery = RuntimePortDiscovery::new();

        // Try multiple times to find an available port (handles race conditions)
        let mut port_found = None;
        for _ in 0..10 {
            let port = discovery.discover_port(None).unwrap();
            if discovery.is_port_available(port) {
                // Try to bind immediately to claim it
                if let Ok(_listener) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
                    // Port is now bound, should be unavailable
                    assert!(!discovery.is_port_available(port));
                    port_found = Some(port);
                    break;
                }
            }
        }

        // Verify we found and tested a port
        assert!(
            port_found.is_some(),
            "Could not find available port after 10 attempts"
        );
    }

    #[test]
    fn test_discover_port_with_preference_available() {
        let discovery = RuntimePortDiscovery::new().with_range(25000..25100);
        let preferred = 25050u16;
        let port = discovery.discover_port(Some(preferred));
        assert!(port.is_ok());
        let port = port.expect("port");
        assert!((25000..25100).contains(&port));
    }

    #[test]
    fn test_discover_port_with_preference_unavailable_finds_alternative() {
        let discovery = RuntimePortDiscovery::new().with_range(17000..17100);
        let port = discovery.discover_port(Some(80));
        assert!(port.is_ok());
        let port = port.expect("port");
        assert_ne!(port, 80);
        assert!(port >= 1024);
    }

    #[test]
    fn test_discover_ports_zero() {
        let discovery = RuntimePortDiscovery::new();
        let ports = discovery.discover_ports(0);
        assert!(ports.is_ok());
        assert!(ports.expect("ports").is_empty());
    }

    #[test]
    fn test_discover_ports_returns_valid_ports() {
        let discovery = RuntimePortDiscovery::new().with_range(28000..28100);
        let ports = discovery.discover_ports(5);
        assert!(ports.is_ok());
        let ports = ports.expect("ports");
        assert_eq!(ports.len(), 5);
        for port in &ports {
            assert!(*port >= 28000 && *port < 28100);
        }
    }

    #[test]
    fn test_discover_port_with_preference_function() {
        let port = discover_port_with_preference(9999);
        assert!(port.is_ok());
        let port = port.expect("port");
        assert!(port >= 1024);
    }

    #[test]
    fn test_runtime_port_discovery_builder_all_interfaces() {
        let discovery = RuntimePortDiscovery::new().all_interfaces();
        let port = discovery.discover_port(None);
        assert!(port.is_ok());
    }

    #[test]
    fn test_runtime_port_discovery_builder_localhost_only() {
        let discovery = RuntimePortDiscovery::new().localhost_only();
        let port = discovery.discover_port(None);
        assert!(port.is_ok());
    }

    #[test]
    fn test_runtime_port_discovery_default_range() {
        let discovery = RuntimePortDiscovery::default();
        let port = discovery.discover_port(None);
        assert!(port.is_ok());
        let port = port.expect("port");
        assert!(port >= 8000);
        assert!(port < 9000);
    }

    #[test]
    fn test_port_error_display() {
        let err = PortError::BindFailed {
            port: 8080,
            reason: "Address already in use".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("8080"));
        assert!(msg.contains("Address already in use"));
    }

    #[test]
    fn test_port_error_no_available() {
        let err = PortError::NoAvailablePorts;
        assert!(!err.to_string().is_empty());
    }
}
