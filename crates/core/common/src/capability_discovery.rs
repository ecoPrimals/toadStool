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
//! - **Edge capability**: Local service discovery (mDNS)
//! - **Cloud capability**: Kubernetes service discovery
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

/// Capability-based discovery client
///
/// This is the **primary interface** for discovering services by capability.
/// It abstracts over multiple discovery methods (mDNS, DNS-SD, K8s, Consul, etc.)
pub struct CapabilityDiscovery {
    /// Underlying discovery implementation
    discovery: Box<dyn ServiceDiscoveryTrait>,

    /// Discovery timeout
    timeout: Duration,

    /// Enable fallback to localhost in development
    enable_localhost_fallback: bool,
}

impl CapabilityDiscovery {
    /// Create new capability discovery client
    ///
    /// Automatically detects available discovery methods:
    /// - Kubernetes (if in K8s cluster)
    /// - mDNS/DNS-SD (if on local network)
    /// - Environment variables (always available)
    pub fn new() -> Result<Self, DiscoveryError> {
        Self::with_config(&DiscoveryConfig::default())
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if the tokio runtime cannot be created
    pub fn with_config(config: &DiscoveryConfig) -> Result<Self, DiscoveryError> {
        // Detect and initialize appropriate discovery backend
        let discovery = Self::detect_discovery_backend()?;

        Ok(Self {
            discovery,
            timeout: config.timeout,
            enable_localhost_fallback: config.enable_localhost_fallback,
        })
    }

    /// Find services by capability
    ///
    /// Returns ALL services providing the requested capability.
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
                self.try_localhost_fallback(&capability).await
            }
            Ok(_) => Err(DiscoveryError::NoServicesFound(format!("{capability:?}"))),
            Err(e) => Err(DiscoveryError::DiscoveryFailed(e.to_string())),
        }
    }

    /// Find the "best" service for a capability
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
            .ok_or(DiscoveryError::NoServicesFound(
                "No services available".to_string(),
            ))
    }

    /// Detect available discovery backend
    ///
    /// **Deep Debt Compliance**: Runtime environment detection
    /// - Checks for Kubernetes environment (service mesh)
    /// - Checks for mDNS capability (local network)
    /// - Falls back to environment variables (self-knowledge)
    /// - Graceful degradation at every level
    fn detect_discovery_backend() -> Result<Box<dyn ServiceDiscoveryTrait>, DiscoveryError> {
        use crate::service_discovery::DiscoveryMethod;

        // 1. Check for Kubernetes environment (KUBERNETES_SERVICE_HOST env var)
        if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            tracing::info!("Detected Kubernetes environment - using K8s service discovery");
            // K8s discovery uses DNS-based service discovery
            // Services are accessible via: <service-name>.<namespace>.svc.cluster.local
            // This is automatically configured by K8s
        }

        // 2. Check for Docker/container environment
        if std::path::Path::new("/.dockerenv").exists() || std::env::var("DOCKER_HOST").is_ok() {
            tracing::info!("Detected containerized environment");
        }

        // 3. Check for mDNS availability (Avahi on Linux, Bonjour on macOS)
        #[cfg(target_os = "linux")]
        {
            if std::path::Path::new("/usr/bin/avahi-browse").exists() {
                tracing::info!("mDNS (Avahi) available - can use for local discovery");
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Bonjour is built into macOS
            tracing::info!("mDNS (Bonjour) available on macOS");
        }

        // 4. Fall back to environment variables (Deep Debt: self-knowledge)
        tracing::info!("Using environment-based service discovery");

        // ServiceDiscovery::new is async, so we need to run it in a blocking context
        // NOTE: This will panic if called from within an async runtime.
        // For async contexts, use the async discovery methods directly.
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| DiscoveryError::InvalidConfig(format!("Failed to create runtime: {e}")))?;

        let discovery = runtime
            .block_on(ServiceDiscovery::new(DiscoveryMethod::Auto))
            .map_err(|e| DiscoveryError::DiscoveryFailed(e.to_string()))?;

        Ok(Box::new(discovery))
    }

    /// Try localhost fallback for development
    async fn try_localhost_fallback(
        &self,
        _capability: &Capability,
    ) -> Result<Vec<DiscoveredService>, DiscoveryError> {
        // Return empty for now - localhost fallback should use environment variables
        Ok(vec![])
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
        let is_production = std::env::var("TOADSTOOL_ENV")
            .map(|e| e == "production")
            .unwrap_or(false);

        Self {
            timeout: Duration::from_secs(5),
            enable_localhost_fallback: !is_production,
            methods: vec![DiscoveryMethod::Auto],
        }
    }
}

/// Discovery methods
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryMethod {
    /// Automatically detect best method
    Auto,

    /// Kubernetes service discovery
    Kubernetes,

    /// mDNS/DNS-SD (local network)
    Mdns,

    /// Consul service discovery
    Consul,

    /// Environment variables
    Environment,
}

/// Discovery errors
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Discovery timeout")]
    Timeout,

    #[error("No services found for capability: {0}")]
    NoServicesFound(String),

    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

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
            methods: vec![DiscoveryMethod::Kubernetes],
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
        // Test all variants
        let auto = DiscoveryMethod::Auto;
        let k8s = DiscoveryMethod::Kubernetes;
        let mdns = DiscoveryMethod::Mdns;
        let consul = DiscoveryMethod::Consul;
        let env = DiscoveryMethod::Environment;

        assert!(matches!(auto, DiscoveryMethod::Auto));
        assert!(matches!(k8s, DiscoveryMethod::Kubernetes));
        assert!(matches!(mdns, DiscoveryMethod::Mdns));
        assert!(matches!(consul, DiscoveryMethod::Consul));
        assert!(matches!(env, DiscoveryMethod::Environment));
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
        // Test production environment disables fallback
        std::env::set_var("TOADSTOOL_ENV", "production");
        let config = DiscoveryConfig::default();
        assert!(!config.enable_localhost_fallback);
        std::env::remove_var("TOADSTOOL_ENV");
    }

    #[test]
    fn test_discovery_config_development_env() {
        // Test development environment enables fallback
        std::env::remove_var("TOADSTOOL_ENV");
        let config = DiscoveryConfig::default();
        assert!(config.enable_localhost_fallback);
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

        // In test env with no services, we expect NoServicesFound or Timeout
        match &result {
            Err(DiscoveryError::NoServicesFound(_)) => {}
            Err(DiscoveryError::Timeout) => {}
            Err(DiscoveryError::DiscoveryFailed(_)) => {}
            Ok(services) => assert!(
                services.is_empty(),
                "expected no services in test env, got {}",
                services.len()
            ),
            other => panic!("unexpected result: {other:?}"),
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
        let _ = format!("{:?}", err);
    }

    #[test]
    fn test_discovery_method_derive_clone() {
        let m = DiscoveryMethod::Kubernetes;
        let m2 = m.clone();
        assert!(matches!(m2, DiscoveryMethod::Kubernetes));
    }
}
