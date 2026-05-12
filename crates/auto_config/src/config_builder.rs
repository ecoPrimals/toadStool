// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ecosystem::{self, DiscoveredServices};
use crate::error::ToadStoolResult;
use crate::hardware::SystemCapabilities;
use crate::intelligent::{self, IntelligentAutoConfig, UsageHints};

/// Advanced configuration builder for fine-grained control
///
/// Use this when you need more control over the auto-configuration process
/// or want to inspect intermediate results.
///
/// # Examples
///
/// ```rust,no_run
/// use toadstool_auto_config::ConfigBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ConfigBuilder::new()
///         .with_hardware_detection(true)
///         .with_ecosystem_discovery(true)
///         .with_performance_optimization(true)
///         .build()
///         .await?;
///
///     println!("Advanced configuration complete!");
///     Ok(())
/// }
/// ```
#[expect(clippy::struct_excessive_bools, reason = "configuration type")]
pub struct ConfigBuilder {
    pub(crate) enable_hardware_detection: bool,
    pub(crate) enable_ecosystem_discovery: bool,
    pub(crate) enable_performance_optimization: bool,
    pub(crate) enable_usage_learning: bool,
    pub(crate) discovery_timeout: std::time::Duration,
}

impl ConfigBuilder {
    const DEFAULT_DISCOVERY_TIMEOUT_SECS: u64 = 30;

    /// Create a new configuration builder with default settings
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enable_hardware_detection: true,
            enable_ecosystem_discovery: true,
            enable_performance_optimization: true,
            enable_usage_learning: true,
            discovery_timeout: std::time::Duration::from_secs(Self::DEFAULT_DISCOVERY_TIMEOUT_SECS),
        }
    }

    /// Enable or disable hardware detection
    #[must_use]
    pub const fn with_hardware_detection(mut self, enable: bool) -> Self {
        self.enable_hardware_detection = enable;
        self
    }

    /// Enable or disable ecosystem service discovery
    #[must_use]
    pub const fn with_ecosystem_discovery(mut self, enable: bool) -> Self {
        self.enable_ecosystem_discovery = enable;
        self
    }

    /// Enable or disable performance optimization
    #[must_use]
    pub const fn with_performance_optimization(mut self, enable: bool) -> Self {
        self.enable_performance_optimization = enable;
        self
    }

    /// Enable or disable usage pattern learning
    #[must_use]
    pub const fn with_usage_learning(mut self, enable: bool) -> Self {
        self.enable_usage_learning = enable;
        self
    }

    /// Set the discovery timeout
    #[must_use]
    pub const fn with_discovery_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.discovery_timeout = timeout;
        self
    }

    /// Build the configuration using the specified settings
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Hardware detection fails and hardware detection is enabled
    /// - Ecosystem discovery times out and ecosystem discovery is enabled
    /// - Performance optimization fails and performance optimization is enabled
    /// - Configuration validation fails
    pub async fn build(self) -> ToadStoolResult<toadstool_config::ToadStoolConfig> {
        use tracing::info;

        info!("🔧 Building custom ToadStool configuration...");

        let mut auto_config = IntelligentAutoConfig::new();

        // Initialize components based on builder settings
        let hardware = if self.enable_hardware_detection {
            info!("🖥️ Hardware detection enabled");
            auto_config.hardware_detector.scan_system().await?
        } else {
            info!("🖥️ Hardware detection disabled - using defaults");
            SystemCapabilities::default()
        };

        let ecosystem = if self.enable_ecosystem_discovery {
            info!("🌐 Ecosystem discovery enabled");
            auto_config.ecosystem_discoverer.discover_services().await?
        } else {
            info!("🌐 Ecosystem discovery disabled");
            DiscoveredServices {
                discovered_services: std::collections::HashMap::new(),
                discovery_summary: ecosystem::DiscoverySummary::default(),
                discovery_timestamp: std::time::SystemTime::now(),
            }
        };

        let platform_config = if self.enable_performance_optimization {
            info!("⚡ Performance optimization enabled");
            auto_config
                .platform_optimizer
                .optimize_for_platform(&hardware)?
        } else {
            info!("⚡ Performance optimization disabled");
            intelligent::PlatformConfig {
                platform_name: std::env::consts::OS.to_string(),
                supported_features: std::collections::HashSet::new(),
                optimizations: Vec::new(),
            }
        };

        let usage_hints = if self.enable_usage_learning {
            info!("📊 Usage pattern learning enabled");
            auto_config.usage_learner.analyze_environment().await?
        } else {
            info!("📊 Usage pattern learning disabled");
            UsageHints::default()
        };

        // Generate the final configuration
        let config = auto_config
            .config_generator
            .generate_optimal_config(&hardware, &platform_config, &ecosystem, &usage_hints)
            .await?;

        info!("✅ Custom configuration build complete");
        Ok(config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_default_values() {
        let b = ConfigBuilder::new();
        assert!(b.enable_hardware_detection);
        assert!(b.enable_ecosystem_discovery);
        assert!(b.enable_performance_optimization);
        assert!(b.enable_usage_learning);
        assert_eq!(b.discovery_timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn builder_default_trait() {
        let b = ConfigBuilder::default();
        assert!(b.enable_hardware_detection);
    }

    #[test]
    fn builder_chain_disables_all() {
        let b = ConfigBuilder::new()
            .with_hardware_detection(false)
            .with_ecosystem_discovery(false)
            .with_performance_optimization(false)
            .with_usage_learning(false)
            .with_discovery_timeout(std::time::Duration::from_secs(5));
        assert!(!b.enable_hardware_detection);
        assert!(!b.enable_ecosystem_discovery);
        assert!(!b.enable_performance_optimization);
        assert!(!b.enable_usage_learning);
        assert_eq!(b.discovery_timeout, std::time::Duration::from_secs(5));
    }

    #[tokio::test]
    async fn build_with_all_disabled() {
        let config = ConfigBuilder::new()
            .with_hardware_detection(false)
            .with_ecosystem_discovery(false)
            .with_performance_optimization(false)
            .with_usage_learning(false)
            .build()
            .await
            .unwrap();
        assert!(config.security.auth.enabled);
    }
}
