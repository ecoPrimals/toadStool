// SPDX-License-Identifier: AGPL-3.0-only
//! Capability-Based Service Discovery - Pure Infant Discovery Pattern
//!
//! This module implements the **pure infant discovery** pattern where ToadStool:
//! 1. Knows only itself (self-knowledge)
//! 2. Discovers other primals at runtime by capabilities
//! 3. Has zero hardcoded primal names or ports
//!
//! ## Philosophy
//!
//! > "Each primal is born knowing nothing except itself.
//! >  It discovers others by what they can do, not who they are."
//!
//! This enables:
//! - **Multi-vendor support**: Any service providing a capability works
//! - **Federation**: Multiple providers for same capability
//! - **Edge capability**: Local service discovery (mDNS via Songbird)
//!
//! ## Evolution (Feb 15, 2026)
//!
//! Service discovery is delegated to Songbird (comms primal). ToadStool only
//! exposes mDNS capability requirements - Songbird handles the actual discovery.
//! Vendor-specific discovery (K8s, Consul, cloud providers) removed.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use toadstool_common::capability_discovery::CapabilityDiscovery;
//! use toadstool_common::primal_identity::{Capability, CryptoCapability};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create discovery engine
//! let discovery = CapabilityDiscovery::new()?;
//!
//! // Find ANY service that can encrypt (agnostic of provider)
//! let crypto_services = discovery
//!     .find_by_capability(Capability::Crypto(CryptoCapability::Encryption))
//!     .await?;
//!
//! // Use the first available (or implement selection logic)
//! if let Some(service) = crypto_services.first() {
//!     println!("Found crypto service: {}", service.name);
//!     // Connect and use...
//! }
//! # Ok(())
//! # }
//! ```

mod types;

#[cfg(test)]
mod tests;

pub use types::{DiscoveryConfig, DiscoveryError, DiscoveryMethod};

use crate::primal_identity::Capability;
use crate::service_discovery::{DiscoveredService, ServiceDiscovery, ServiceDiscoveryTrait};
use std::time::Duration;

/// PATH-based binary lookup (pure Rust, no external `which` crate).
#[cfg(target_os = "linux")]
fn which_in_path(binary: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path_var| {
        std::env::split_paths(&path_var).any(|dir| dir.join(binary).is_file())
    })
}

/// Capability-based discovery client
///
/// This is the **primary interface** for discovering services by capability.
/// Service discovery is delegated to Songbird (comms primal) via mDNS.
pub struct CapabilityDiscovery {
    /// Underlying discovery implementation
    discovery: Box<dyn ServiceDiscoveryTrait>,

    /// Discovery timeout
    timeout: Duration,

    /// Enable fallback to localhost in development
    enable_localhost_fallback: bool,
}

impl CapabilityDiscovery {
    /// Create new capability discovery client (sync bridge).
    ///
    /// Automatically detects available discovery methods:
    /// - mDNS/DNS-SD via Songbird (if on local network)
    /// - Environment variables (always available)
    ///
    /// Prefer [`new_async`](Self::new_async) when calling from async contexts.
    ///
    /// # Errors
    ///
    /// Returns error if discovery backend cannot be initialized.
    pub fn new() -> Result<Self, DiscoveryError> {
        Self::with_config(&DiscoveryConfig::default())
    }

    /// Create new capability discovery client (async, no runtime bridge).
    ///
    /// # Errors
    ///
    /// Returns error if discovery backend cannot be initialized.
    pub async fn new_async() -> Result<Self, DiscoveryError> {
        Self::with_config_async(&DiscoveryConfig::default()).await
    }

    /// Create with custom configuration (sync bridge).
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if the discovery backend cannot be initialized.
    pub fn with_config(config: &DiscoveryConfig) -> Result<Self, DiscoveryError> {
        let discovery = Self::detect_discovery_backend()?;

        Ok(Self {
            discovery,
            timeout: config.timeout,
            enable_localhost_fallback: config.enable_localhost_fallback,
        })
    }

    /// Create with custom configuration (async, no runtime bridge).
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if the discovery backend cannot be initialized.
    pub async fn with_config_async(config: &DiscoveryConfig) -> Result<Self, DiscoveryError> {
        let discovery = Self::detect_discovery_backend_async().await?;

        Ok(Self {
            discovery,
            timeout: config.timeout,
            enable_localhost_fallback: config.enable_localhost_fallback,
        })
    }

    /// Find services by capability
    ///
    /// Returns ALL services providing the requested capability.
    ///
    /// # Errors
    ///
    /// Returns error if timeout expires, discovery fails, or no services found (when localhost fallback disabled).
    /// Caller can choose based on additional criteria (latency, trust level, etc.)
    ///
    /// ## Multi-Provider Example
    ///
    /// ```rust,no_run
    /// # use toadstool_common::capability_discovery::CapabilityDiscovery;
    /// # use toadstool_common::primal_identity::{Capability, CryptoCapability};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = CapabilityDiscovery::new()?;
    ///
    /// // Find ALL crypto providers
    /// let providers = discovery
    ///     .find_by_capability(Capability::Crypto(CryptoCapability::Encryption))
    ///     .await?;
    ///
    /// // Select based on criteria (metadata can contain latency, trust info)
    /// let local_provider = providers.iter()
    ///     .filter(|s| s.healthy)
    ///     .next();
    ///
    /// let latest_version = providers.iter()
    ///     .max_by(|a, b| a.version.cmp(&b.version));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_capability(
        &self,
        capability: Capability,
    ) -> Result<Vec<DiscoveredService>, DiscoveryError> {
        // Use timeout
        let result = tokio::time::timeout(
            self.timeout,
            self.discovery.find_services_by_capability(&capability),
        )
        .await
        .map_err(|_| DiscoveryError::Timeout)?;

        match result {
            Ok(services) if !services.is_empty() => Ok(services),
            Ok(_) if self.enable_localhost_fallback => {
                // Fallback for development
                Ok(Self::try_localhost_fallback(&capability))
            }
            Ok(_) => Err(DiscoveryError::NoServicesFound(format!("{capability:?}"))),
            Err(e) => Err(DiscoveryError::DiscoveryFailed(e.to_string())),
        }
    }

    /// Find the "best" service for a capability
    ///
    /// # Errors
    ///
    /// Returns error if [`find_by_capability`](Self::find_by_capability) fails or no services available.
    ///
    /// Selection criteria (in order):
    /// 1. Healthy services preferred
    /// 2. Most recently seen (fresher discovery)
    /// 3. Metadata-based trust level (if available)
    pub async fn find_best(
        &self,
        capability: Capability,
    ) -> Result<DiscoveredService, DiscoveryError> {
        let services = self.find_by_capability(capability).await?;

        services
            .into_iter()
            .max_by(|a, b| {
                // Healthy services first
                let health_cmp = a.healthy.cmp(&b.healthy);
                if health_cmp != std::cmp::Ordering::Equal {
                    return health_cmp;
                }

                // Then by freshness (more recent is better)
                a.last_seen.cmp(&b.last_seen)
            })
            .ok_or_else(|| DiscoveryError::NoServicesFound("No services available".to_string()))
    }

    /// Async detection of available discovery backend.
    ///
    /// **Deep Debt Compliance**: Runtime environment detection
    /// - Checks for mDNS capability (local network via Songbird)
    /// - Falls back to environment variables (self-knowledge)
    /// - Graceful degradation at every level
    ///
    /// ## Evolution (Mar 29, 2026)
    ///
    /// Removed nested `Runtime::new()` + `block_on` anti-pattern.
    /// Now natively async. Sync callers should use `new()` which
    /// detects the current runtime or creates one safely.
    async fn detect_discovery_backend_async()
    -> Result<Box<dyn ServiceDiscoveryTrait>, DiscoveryError> {
        use crate::service_discovery::DiscoveryMethod;

        #[cfg(target_os = "linux")]
        {
            if which_in_path("avahi-browse") {
                tracing::info!("mDNS (Avahi) available - Songbird can use for local discovery");
            }
        }

        #[cfg(target_os = "macos")]
        {
            tracing::info!("mDNS (Bonjour) available on macOS - Songbird can use for discovery");
        }

        tracing::info!("Using environment-based service discovery");

        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
            .await
            .map_err(|e| DiscoveryError::DiscoveryFailed(e.to_string()))?;

        Ok(Box::new(discovery))
    }

    /// Sync bridge for discovery backend initialization.
    ///
    /// Tries to use an existing tokio runtime handle (safe from async contexts).
    /// Falls back to creating a lightweight current-thread runtime when no
    /// runtime is active.
    fn detect_discovery_backend() -> Result<Box<dyn ServiceDiscoveryTrait>, DiscoveryError> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            std::thread::scope(|s| {
                s.spawn(|| handle.block_on(Self::detect_discovery_backend_async()))
                    .join()
                    .map_err(|_| {
                        DiscoveryError::DiscoveryFailed("discovery thread panicked".to_string())
                    })?
            })
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    DiscoveryError::InvalidConfig(format!("Failed to create runtime: {e}"))
                })?;
            rt.block_on(Self::detect_discovery_backend_async())
        }
    }

    /// Try localhost fallback for development
    const fn try_localhost_fallback(_capability: &Capability) -> Vec<DiscoveredService> {
        // Return empty for now - localhost fallback should use environment variables
        vec![]
    }
}
