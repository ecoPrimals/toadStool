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

use crate::primal_identity::Capability;
use crate::service_discovery::{DiscoveredService, ServiceDiscovery, ServiceDiscoveryTrait};
use std::time::Duration;
use thiserror::Error;

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
/// ## Evolution (Feb 15, 2026)
///
/// Vendor-specific methods (Kubernetes, Consul) are deprecated.
/// Service discovery is delegated to Songbird (comms primal).
/// ToadStool only supports mDNS (via Songbird) and environment variables.
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryMethod {
    /// Automatically detect best method
    Auto,

    /// mDNS/DNS-SD (local network via Songbird)
    Mdns,

    /// Environment variables (self-knowledge)
    Environment,

    /// Kubernetes service discovery (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use mDNS via Songbird instead")]
    Kubernetes,

    /// Consul service discovery (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use mDNS via Songbird instead")]
    Consul,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(5));
        // Fallback enabled in non-production
        assert!(config.enable_localhost_fallback);
    }

    #[test]
    fn test_discovery_config_custom() {
        let config = DiscoveryConfig {
            timeout: Duration::from_secs(10),
            enable_localhost_fallback: false,
            methods: vec![DiscoveryMethod::Mdns],
        };
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert!(!config.enable_localhost_fallback);
    }

    #[test]
    fn test_discovery_method_copy() {
        let method1 = DiscoveryMethod::Auto;
        let method2 = method1; // Copy
        assert!(matches!(method1, DiscoveryMethod::Auto));
        assert!(matches!(method2, DiscoveryMethod::Auto));
    }

    #[test]
    fn test_discovery_method_variants() {
        // Test non-deprecated variants
        let auto = DiscoveryMethod::Auto;
        let mdns = DiscoveryMethod::Mdns;
        let env = DiscoveryMethod::Environment;

        assert!(matches!(auto, DiscoveryMethod::Auto));
        assert!(matches!(mdns, DiscoveryMethod::Mdns));
        assert!(matches!(env, DiscoveryMethod::Environment));
    }

    #[test]
    #[allow(deprecated)]
    fn test_discovery_method_deprecated_variants() {
        // Test deprecated variants still exist for backward compatibility
        let k8s = DiscoveryMethod::Kubernetes;
        let consul = DiscoveryMethod::Consul;

        assert!(matches!(k8s, DiscoveryMethod::Kubernetes));
        assert!(matches!(consul, DiscoveryMethod::Consul));
    }

    #[test]
    fn test_discovery_error_timeout() {
        let err = DiscoveryError::Timeout;
        assert_eq!(err.to_string(), "Discovery timeout");
    }

    #[test]
    fn test_discovery_error_no_services() {
        let err = DiscoveryError::NoServicesFound("test_capability".to_string());
        assert!(err.to_string().contains("test_capability"));
    }

    #[test]
    fn test_discovery_error_failed() {
        let err = DiscoveryError::DiscoveryFailed("network error".to_string());
        assert!(err.to_string().contains("network error"));
    }

    #[test]
    fn test_discovery_error_invalid_config() {
        let err = DiscoveryError::InvalidConfig("bad config".to_string());
        assert!(err.to_string().contains("bad config"));
    }

    #[test]
    fn test_discovery_config_production_env() {
        temp_env::with_var("TOADSTOOL_ENV", Some("production"), || {
            let config = DiscoveryConfig::default();
            assert!(!config.enable_localhost_fallback);
        });
    }

    #[test]
    fn test_discovery_config_development_env() {
        temp_env::with_var_unset("TOADSTOOL_ENV", || {
            let config = DiscoveryConfig::default();
            assert!(config.enable_localhost_fallback);
        });
    }

    #[test]
    fn test_discovery_config_builder_pattern() {
        let config = DiscoveryConfig {
            timeout: Duration::from_millis(100),
            enable_localhost_fallback: true,
            methods: vec![DiscoveryMethod::Mdns, DiscoveryMethod::Environment],
        };
        assert_eq!(config.timeout, Duration::from_millis(100));
        assert!(config.enable_localhost_fallback);
        assert_eq!(config.methods.len(), 2);
    }

    #[test]
    fn test_discovery_config_clone() {
        let config1 = DiscoveryConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.timeout, config2.timeout);
        assert_eq!(
            config1.enable_localhost_fallback,
            config2.enable_localhost_fallback
        );
    }

    // Note: CapabilityDiscovery::new() and find_by_capability use block_on internally,
    // which panics when called from within a tokio runtime (e.g. #[tokio::test]).
    // Integration tests that need the full discovery stack should run outside tokio.
    #[test]
    fn test_capability_discovery_new_from_sync() {
        let result = std::thread::spawn(CapabilityDiscovery::new)
            .join()
            .expect("thread should not panic");
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_discovery_with_config() {
        let config = DiscoveryConfig {
            timeout: Duration::from_millis(50),
            enable_localhost_fallback: false,
            methods: vec![DiscoveryMethod::Environment],
        };
        let result = CapabilityDiscovery::with_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_by_capability_no_services_in_separate_thread() {
        use crate::primal_identity::{Capability, CryptoCapability};

        let result = std::thread::spawn(|| {
            let config = DiscoveryConfig {
                timeout: Duration::from_millis(100),
                enable_localhost_fallback: false,
                methods: vec![DiscoveryMethod::Environment],
            };
            let discovery = CapabilityDiscovery::with_config(&config).expect("discovery");
            let runtime = tokio::runtime::Runtime::new().expect("runtime");
            runtime.block_on(
                discovery.find_by_capability(Capability::Crypto(CryptoCapability::Encryption)),
            )
        })
        .join()
        .expect("thread ok");

        // In test env with no services, we expect NoServicesFound, Timeout, DiscoveryFailed, or InvalidConfig
        match &result {
            Err(
                DiscoveryError::NoServicesFound(_)
                | DiscoveryError::Timeout
                | DiscoveryError::DiscoveryFailed(_)
                | DiscoveryError::InvalidConfig(_),
            ) => {}
            Ok(services) => assert!(
                services.is_empty(),
                "expected no services in test env, got {}",
                services.len()
            ),
        }
    }

    #[test]
    fn test_find_by_capability_with_localhost_fallback() {
        use crate::primal_identity::{Capability, CryptoCapability};

        let result = std::thread::spawn(|| {
            let config = DiscoveryConfig {
                timeout: Duration::from_millis(100),
                enable_localhost_fallback: true,
                methods: vec![DiscoveryMethod::Environment],
            };
            let discovery = CapabilityDiscovery::with_config(&config).expect("discovery");
            let runtime = tokio::runtime::Runtime::new().expect("runtime");
            runtime.block_on(
                discovery.find_by_capability(Capability::Crypto(CryptoCapability::Encryption)),
            )
        })
        .join()
        .expect("thread ok");

        // With fallback enabled, empty discovery returns Ok(vec![]) from try_localhost_fallback
        match &result {
            Ok(services) => assert!(services.is_empty()),
            Err(e) => assert!(
                matches!(
                    e,
                    DiscoveryError::NoServicesFound(_)
                        | DiscoveryError::Timeout
                        | DiscoveryError::DiscoveryFailed(_)
                ),
                "unexpected error: {e}"
            ),
        }
    }

    #[test]
    fn test_discovery_error_display_all_variants() {
        let timeout_err = DiscoveryError::Timeout;
        assert_eq!(timeout_err.to_string(), "Discovery timeout");

        let no_services =
            DiscoveryError::NoServicesFound("Capability::Crypto(Encryption)".to_string());
        assert!(no_services.to_string().contains("Crypto"));
        assert!(no_services.to_string().contains("Encryption"));

        let failed = DiscoveryError::DiscoveryFailed("network down".to_string());
        assert!(failed.to_string().contains("network down"));

        let invalid = DiscoveryError::InvalidConfig("bad".to_string());
        assert!(invalid.to_string().contains("bad"));
    }

    #[test]
    fn test_discovery_error_is_std_error() {
        use std::error::Error;
        let err = DiscoveryError::Timeout;
        assert!(err.source().is_none());
        let _ = format!("{err:?}");
    }

    #[test]
    fn test_discovery_method_derive_clone() {
        let m = DiscoveryMethod::Mdns;
        let m2 = m;
        assert!(matches!(m2, DiscoveryMethod::Mdns));
    }

    // ═══════════════════════════════════════════════════════════════════
    // Additional tests for capability discovery logic and error paths
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_find_best_empty_services_returns_error() {
        use crate::primal_identity::{Capability, CryptoCapability};

        let result = std::thread::spawn(|| {
            let config = DiscoveryConfig {
                timeout: Duration::from_millis(50),
                enable_localhost_fallback: false,
                methods: vec![DiscoveryMethod::Environment],
            };
            let discovery = CapabilityDiscovery::with_config(&config).expect("discovery");
            let runtime = tokio::runtime::Runtime::new().expect("runtime");
            runtime.block_on(discovery.find_best(Capability::Crypto(CryptoCapability::Encryption)))
        })
        .join()
        .expect("thread ok");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                DiscoveryError::NoServicesFound(_)
                    | DiscoveryError::Timeout
                    | DiscoveryError::DiscoveryFailed(_)
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_discovery_config_default_methods() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.methods.len(), 1);
        assert!(matches!(config.methods[0], DiscoveryMethod::Auto));
    }

    #[test]
    fn test_discovery_error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DiscoveryError>();
    }

    #[test]
    fn test_discovery_config_debug() {
        let config = DiscoveryConfig::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("DiscoveryConfig"));
    }

    #[test]
    fn test_discovery_method_debug() {
        let m = DiscoveryMethod::Auto;
        let debug_str = format!("{m:?}");
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_capability_discovery_new_creates_valid_instance() {
        let discovery = CapabilityDiscovery::new().expect("discovery");
        assert!(std::mem::size_of_val(&discovery) > 0);
    }

    #[test]
    fn test_capability_discovery_with_config_succeeds() {
        let config = DiscoveryConfig {
            timeout: Duration::from_secs(30),
            enable_localhost_fallback: true,
            methods: vec![DiscoveryMethod::Auto],
        };
        let discovery = CapabilityDiscovery::with_config(&config);
        assert!(discovery.is_ok());
    }

    #[test]
    fn test_try_localhost_fallback_returns_empty() {
        use crate::primal_identity::{Capability, CryptoCapability};

        let fallback = CapabilityDiscovery::try_localhost_fallback(&Capability::Crypto(
            CryptoCapability::Encryption,
        ));
        assert!(fallback.is_empty());
    }
}
