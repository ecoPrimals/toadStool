// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use thiserror::Error;

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery timeout
    pub timeout: Duration,

    /// Enable localhost fallback in development
    pub enable_localhost_fallback: bool,

    /// Discovery methods to try
    pub methods: Vec<DiscoveryMethod>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        let is_production = std::env::var("TOADSTOOL_ENV").is_ok_and(|e| e == "production");

        Self {
            timeout: Duration::from_secs(5),
            enable_localhost_fallback: !is_production,
            methods: vec![DiscoveryMethod::Auto],
        }
    }
}

/// Discovery methods
///
/// Service discovery is delegated to the coordination service (comms layer).
/// ToadStool only supports mDNS (via that layer) and environment variables.
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryMethod {
    /// Automatically detect best method
    Auto,

    /// mDNS/DNS-SD (local network via coordination service)
    Mdns,

    /// Environment variables (self-knowledge)
    Environment,
}

/// Discovery errors
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// Discovery operation exceeded timeout
    #[error("Discovery timeout")]
    Timeout,

    /// No services advertising the capability were found
    #[error("No services found for capability: {0}")]
    NoServicesFound(String),

    /// Discovery backend failed
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

    /// Configuration was invalid
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
