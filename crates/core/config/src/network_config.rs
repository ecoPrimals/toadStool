// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Centralized network configuration with environment overrides
//!
//! This module eliminates hardcoded ports/addresses with:
//! - Environment variable overrides
//! - Sane defaults for development
//! - Production-ready configuration
//! - Zero hardcoded knowledge

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Network configuration - centralized, environment-aware, zero hardcoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Our listen address
    pub listen_address: IpAddr,

    /// Our primary service port
    pub service_port: u16,

    /// API endpoint port
    pub api_port: u16,

    /// Metrics/monitoring port
    pub metrics_port: u16,

    /// Health check port
    pub health_port: u16,

    /// Discovery service endpoints (where to announce/discover services)
    pub discovery_endpoints: Vec<String>,

    /// Enable mDNS discovery (for local networks)
    pub enable_mdns: bool,

    /// Network bind mode
    pub bind_mode: BindMode,
}

/// How to bind to network interfaces
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BindMode {
    /// Bind to localhost only (development)
    Localhost,

    /// Bind to all interfaces (production)
    AllInterfaces,

    /// Bind to specific address (advanced)
    Specific,
}

impl std::str::FromStr for BindMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "localhost" | "local" => Ok(Self::Localhost),
            "all" | "allinterfaces" | "0.0.0.0" => Ok(Self::AllInterfaces),
            "specific" => Ok(Self::Specific),
            _ => Err(format!("Invalid bind mode: {s}")),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        // Deep Debt: Prefer standard ports, but these are just preferences
        // Actual runtime will check availability and adjust
        Self {
            listen_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            service_port: 8080,
            api_port: 8080,
            metrics_port: 9090,
            health_port: 8081,
            // Deep Debt: No hardcoded discovery endpoints by default
            // Services should be discovered via mDNS or provided via environment
            discovery_endpoints: vec![],
            enable_mdns: true, // Rely on mDNS for discovery
            bind_mode: BindMode::Localhost,
        }
    }
}

impl NetworkConfig {
    /// Load configuration from environment variables with fallback to defaults
    ///
    /// # Environment Variables
    /// - `TOADSTOOL_LISTEN_ADDRESS` - IP address to bind to
    /// - `TOADSTOOL_SERVICE_PORT` - Main service port
    /// - `TOADSTOOL_API_PORT` - API endpoint port
    /// - `TOADSTOOL_METRICS_PORT` - Metrics/monitoring port
    /// - `TOADSTOOL_HEALTH_PORT` - Health check port
    /// - `TOADSTOOL_DISCOVERY_ENDPOINTS` - Comma-separated discovery endpoints
    /// - `TOADSTOOL_ENABLE_MDNS` - Enable mDNS (true/false)
    /// - `TOADSTOOL_BIND_MODE` - Bind mode (localhost/all/specific)
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            listen_address: env_var_or("TOADSTOOL_LISTEN_ADDRESS", IpAddr::V4(Ipv4Addr::LOCALHOST)),
            service_port: env_var_or("TOADSTOOL_SERVICE_PORT", 8080),
            api_port: env_var_or("TOADSTOOL_API_PORT", 8080),
            metrics_port: env_var_or("TOADSTOOL_METRICS_PORT", 9090),
            health_port: env_var_or("TOADSTOOL_HEALTH_PORT", 8081),
            // Deep Debt: Only use discovery endpoints from environment
            // No hardcoded fallbacks - rely on mDNS or explicit configuration
            discovery_endpoints: env_var_list_or("TOADSTOOL_DISCOVERY_ENDPOINTS", vec![]),
            enable_mdns: env_var_or("TOADSTOOL_ENABLE_MDNS", true),
            bind_mode: env_var_or("TOADSTOOL_BIND_MODE", BindMode::Localhost),
        }
    }

    /// Get service socket address
    #[must_use]
    pub fn service_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.service_port)
    }

    /// Get API socket address
    #[must_use]
    pub fn api_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.api_port)
    }

    /// Get metrics socket address
    #[must_use]
    pub fn metrics_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.metrics_port)
    }

    /// Get health check socket address
    #[must_use]
    pub fn health_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.health_port)
    }

    /// Create production configuration (bind to all interfaces)
    #[must_use]
    pub fn production() -> Self {
        Self {
            listen_address: IpAddr::V4(Ipv4Addr::UNSPECIFIED), // 0.0.0.0
            bind_mode: BindMode::AllInterfaces,
            enable_mdns: false, // Disable mDNS in production
            ..Self::from_env()
        }
    }

    /// Create development configuration (localhost only)
    #[must_use]
    pub fn development() -> Self {
        Self {
            listen_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            bind_mode: BindMode::Localhost,
            enable_mdns: true,
            ..Self::from_env()
        }
    }

    /// Create test configuration (random available ports)
    #[must_use]
    pub fn test() -> Self {
        Self {
            listen_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            service_port: 0, // OS assigns available port
            api_port: 0,
            metrics_port: 0,
            health_port: 0,
            discovery_endpoints: vec![],
            enable_mdns: false,
            bind_mode: BindMode::Localhost,
        }
    }
}

/// Helper to get environment variable with type parsing and fallback
fn env_var_or<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Helper to get comma-separated list from environment variable
fn env_var_list_or(key: &str, default: Vec<String>) -> Vec<String> {
    std::env::var(key)
        .ok()
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .filter(|list: &Vec<String>| !list.is_empty())
        .unwrap_or(default)
}

/// Service endpoint builder - creates endpoint URLs from configuration
pub struct EndpointBuilder {
    config: NetworkConfig,
}

impl EndpointBuilder {
    /// Create a new endpoint builder from configuration
    #[must_use]
    pub fn new(config: NetworkConfig) -> Self {
        Self { config }
    }

    /// Build service endpoint URL
    #[must_use]
    pub fn service_url(&self) -> String {
        self.build_url(self.config.service_port)
    }

    /// Build API endpoint URL
    #[must_use]
    pub fn api_url(&self) -> String {
        self.build_url(self.config.api_port)
    }

    /// Build metrics endpoint URL
    #[must_use]
    pub fn metrics_url(&self) -> String {
        self.build_url(self.config.metrics_port)
    }

    /// Build health check endpoint URL
    #[must_use]
    pub fn health_url(&self) -> String {
        self.build_url(self.config.health_port)
    }

    fn build_url(&self, port: u16) -> String {
        let host = match self.config.bind_mode {
            BindMode::Localhost => "localhost",
            BindMode::AllInterfaces => {
                // Use hostname or default to localhost
                std::env::var("HOSTNAME")
                    .or_else(|_| std::env::var("HOST"))
                    .unwrap_or_else(|_| "localhost".to_string())
                    .leak()
            }
            BindMode::Specific => {
                // Use the specific address
                match self.config.listen_address {
                    IpAddr::V4(addr) => {
                        if addr.is_loopback() {
                            "localhost"
                        } else {
                            Box::leak(addr.to_string().into_boxed_str())
                        }
                    }
                    IpAddr::V6(addr) => Box::leak(format!("[{addr}]").into_boxed_str()),
                }
            }
        };

        format!("http://{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.service_port, 8080);
        assert_eq!(config.bind_mode, BindMode::Localhost);
    }

    #[test]
    fn test_production_config() {
        let config = NetworkConfig::production();
        assert_eq!(config.bind_mode, BindMode::AllInterfaces);
        assert!(!config.enable_mdns);
    }

    #[test]
    fn test_development_config() {
        let config = NetworkConfig::development();
        assert_eq!(config.bind_mode, BindMode::Localhost);
        assert!(config.enable_mdns);
    }

    #[test]
    fn test_test_config() {
        let config = NetworkConfig::test();
        assert_eq!(config.service_port, 0); // Random port
        assert_eq!(config.discovery_endpoints.len(), 0);
    }

    #[test]
    fn test_endpoint_builder() {
        let config = NetworkConfig::default();
        let builder = EndpointBuilder::new(config);

        assert_eq!(builder.service_url(), "http://localhost:8080");
        assert_eq!(builder.api_url(), "http://localhost:8080");
        assert_eq!(builder.metrics_url(), "http://localhost:9090");
        assert_eq!(builder.health_url(), "http://localhost:8081");
    }

    #[test]
    fn test_env_var_override() {
        std::env::set_var("TOADSTOOL_SERVICE_PORT", "9999");
        let config = NetworkConfig::from_env();
        assert_eq!(config.service_port, 9999);
        std::env::remove_var("TOADSTOOL_SERVICE_PORT");
    }
}
