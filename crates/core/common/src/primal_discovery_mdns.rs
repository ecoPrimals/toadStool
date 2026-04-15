// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration adapter between `primal_discovery` and mDNS
//!
//! This module provides mDNS-SD (multicast DNS service discovery) for finding
//! Primal services on the local network. Pure Rust implementation using `mdns-sd`.
//!
//! ## Service Type
//!
//! Uses `_toadstool._tcp.local.` as the service type, matching the ecosystem standard.
//!
//! ## Capability-Based Discovery
//!
//! Services advertise capabilities via TXT records:
//! - `cap_{name}={version}` - capability with version
//! - `cap_{name}_features={comma,separated}` - optional features
//! - `instance_id={uuid}` - unique service instance
//! - `primal_type={type}` - type of primal (discovered by capability at runtime)

use crate::constants::network::HTTP_PROTOCOL;
use crate::primal_discovery::{
    DiscoveryConfig, DiscoveryError, DiscoveryMethod, PrimalEndpoint, TrustLevel,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// mDNS service type for Toadstool ecosystem
pub const TOADSTOOL_SERVICE_TYPE: &str = "_toadstool._tcp.local.";

/// Default discovery timeout.
/// Test builds use a short window — no real mDNS services exist in CI.
const DEFAULT_DISCOVERY_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_millis(50)
} else {
    Duration::from_secs(3)
};

/// Adapter to integrate mDNS with primal discovery
///
/// Provides real mDNS-SD discovery for finding Primal services on the local network.
pub struct MdnsAdapter {
    /// mDNS daemon for browse operations
    daemon: ServiceDaemon,
    /// Discovery configuration
    config: Arc<DiscoveryConfig>,
    /// Discovery timeout
    timeout: Duration,
}

impl MdnsAdapter {
    /// Create new mDNS adapter
    ///
    /// Initializes the mDNS daemon for service discovery.
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if the mDNS daemon cannot be created.
    pub fn new(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| DiscoveryError::MDnsError(format!("Failed to create mDNS daemon: {e}")))?;

        Ok(Self {
            daemon,
            config: Arc::new(config),
            timeout: DEFAULT_DISCOVERY_TIMEOUT,
        })
    }

    /// Create with custom timeout
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if the mDNS daemon cannot be created.
    pub fn with_timeout(
        config: DiscoveryConfig,
        timeout: Duration,
    ) -> Result<Self, DiscoveryError> {
        let mut adapter = Self::new(config)?;
        adapter.timeout = timeout;
        Ok(adapter)
    }

    /// Discover services via mDNS that have the specified capability
    ///
    /// Performs a real mDNS browse operation and filters by capability.
    /// Returns services that advertise the requested capability.
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if mDNS browse fails.
    pub fn discover(&self, capability: &str) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        tracing::debug!(
            "Starting mDNS discovery for capability '{}' (timeout: {:?})",
            capability,
            self.timeout
        );

        // Start browsing for services
        let receiver = self.daemon.browse(TOADSTOOL_SERVICE_TYPE).map_err(|e| {
            DiscoveryError::MDnsError(format!("Failed to browse mDNS services: {e}"))
        })?;

        let mut discovered = Vec::new();
        let deadline = std::time::Instant::now() + self.timeout;

        // Collect services until timeout
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    // Parse capabilities from TXT properties
                    let mut capabilities = Vec::new();
                    let mut instance_id = None;

                    for prop in info.get_properties().iter() {
                        let key = prop.key();

                        // Extract instance_id
                        if key == "instance_id" {
                            instance_id = info
                                .get_property_val_str(key)
                                .map(std::string::ToString::to_string);
                        }

                        // Extract capabilities (cap_{name}={version})
                        if let Some(cap_name) = key.strip_prefix("cap_") {
                            if !cap_name.ends_with("_features") {
                                capabilities.push(cap_name.to_string());
                            }
                        }
                    }

                    // Check if this service has the requested capability
                    if capabilities.iter().any(|c| c == capability) {
                        // Build endpoint URL
                        let addresses = info.get_addresses();
                        let host = addresses
                            .iter()
                            .next()
                            .map_or_else(|| info.get_hostname().to_string(), ToString::to_string);
                        let port = info.get_port();
                        let url = format!("{HTTP_PROTOCOL}{host}:{port}");

                        let service_id = instance_id
                            .unwrap_or_else(|| format!("{}:{}", info.get_hostname(), port));

                        let endpoint =
                            convert_mdns_service_to_endpoint(service_id, capabilities, url);

                        tracing::debug!(
                            "Discovered service '{}' with capability '{}'",
                            endpoint.service_id,
                            capability
                        );
                        discovered.push(endpoint);
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_, full_name)) => {
                    tracing::debug!("Service removed: {}", full_name);
                }
                Ok(_) => {}      // Ignore other events
                Err(_) => break, // Timeout or channel closed
            }
        }

        tracing::info!(
            "mDNS discovery complete: found {} services with capability '{}'",
            discovered.len(),
            capability
        );

        Ok(discovered)
    }

    /// Discover all services via mDNS (regardless of capability)
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if mDNS browse fails.
    pub fn discover_all(&self) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        tracing::debug!(
            "Starting mDNS discovery for all services (timeout: {:?})",
            self.timeout
        );

        let receiver = self.daemon.browse(TOADSTOOL_SERVICE_TYPE).map_err(|e| {
            DiscoveryError::MDnsError(format!("Failed to browse mDNS services: {e}"))
        })?;

        let mut discovered = Vec::new();
        let deadline = std::time::Instant::now() + self.timeout;

        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let mut capabilities = Vec::new();
                    let mut instance_id = None;

                    for prop in info.get_properties().iter() {
                        let key = prop.key();
                        if key == "instance_id" {
                            instance_id = info
                                .get_property_val_str(key)
                                .map(std::string::ToString::to_string);
                        }
                        if let Some(cap_name) = key.strip_prefix("cap_") {
                            if !cap_name.ends_with("_features") {
                                capabilities.push(cap_name.to_string());
                            }
                        }
                    }

                    let addresses = info.get_addresses();
                    let host = addresses
                        .iter()
                        .next()
                        .map_or_else(|| info.get_hostname().to_string(), ToString::to_string);
                    let port = info.get_port();
                    let url = format!("{HTTP_PROTOCOL}{host}:{port}");

                    let service_id =
                        instance_id.unwrap_or_else(|| format!("{}:{}", info.get_hostname(), port));

                    discovered.push(convert_mdns_service_to_endpoint(
                        service_id,
                        capabilities,
                        url,
                    ));
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        tracing::info!(
            "mDNS discovery complete: found {} total services",
            discovered.len()
        );
        Ok(discovered)
    }

    /// Get the configured timeout
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &DiscoveryConfig {
        &self.config
    }
}

/// Helper to convert mDNS discovered services to `PrimalEndpoints`
fn convert_mdns_service_to_endpoint(
    service_id: String,
    capabilities: Vec<String>,
    url: String,
) -> PrimalEndpoint {
    PrimalEndpoint {
        service_id,
        capabilities,
        url,
        trust_level: TrustLevel::Local, // mDNS is local network
        discovered_via: DiscoveryMethod::MDns,
        discovered_at: Instant::now(),
        last_seen: Instant::now(),
        latency_ms: 0, // Initial: updated on first health check ping
    }
}

#[cfg(test)]
#[path = "primal_discovery_mdns_tests.rs"]
mod tests;
