// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::needless_raw_string_hashes,
    clippy::unused_async,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::trivially_copy_pass_by_ref,
    clippy::struct_excessive_bools,
    clippy::match_same_arms,
    clippy::implicit_hasher,
    clippy::pub_underscore_fields
)]

//! # `ToadStool` Auto-Configuration Library
//!
//! Zero-touch, grandma-friendly auto-configuration system for `ToadStool` Universal Compute Platform.
//! This library makes `ToadStool` so easy that anyone can use it with zero configuration
//! while being perfectly AI-friendly.
//!
//! ## Core Philosophy
//!
//! - **🎯 Zero-Touch**: Works perfectly out-of-the-box with zero configuration
//! - **🧠 Intelligent**: Automatically detects optimal settings for any environment
//! - **👵 Grandma-Friendly**: So simple that anyone can use it
//! - **🤖 AI-Native**: Perfect integration with Squirrel MCP AI interface
//! - **🔄 Self-Healing**: Adapts to changing conditions automatically
//!
//! ## Usage
//!
//! ### Basic Auto-Configuration
//!
//! ```rust,ignore
//! // Example usage (API may change)
//! use toadstool_auto_config::IntelligentAutoConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Zero-touch startup - just works!
//!     let config = IntelligentAutoConfig::auto_configure().await?;
//!     
//!     // Configuration is now ready to use
//!     println!("🎉 ToadStool auto-configured successfully!");
//!     Ok(())
//! }
//! ```
//!
//! ### Advanced Configuration
//!
//! ```rust,no_run
//! use toadstool_auto_config::{
//!     IntelligentAutoConfig, HardwareDetector, EcosystemDiscoverer
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create components separately for fine-grained control
//!     let mut auto_config = IntelligentAutoConfig::new();
//!     let mut hardware_detector = HardwareDetector::new();
//!     let mut ecosystem_discoverer = EcosystemDiscoverer::new();
//!     
//!     // Scan system capabilities
//!     let hardware = hardware_detector.scan_system().await?;
//!     println!("System: {} cores, {:.1}GB RAM, {} GPUs",
//!              hardware.cpu_cores, hardware.memory_gb, hardware.gpu_count);
//!     
//!     // Discover ecosystem services
//!     let ecosystem = ecosystem_discoverer.discover_services().await?;
//!     println!("Found {} ecosystem services", ecosystem.discovered_services.len());
//!     
//!     // Generate configuration
//!     let config = IntelligentAutoConfig::auto_configure().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! ### 🧠 Intelligent Auto-Configuration
//! - **Hardware Detection**: Comprehensive CPU, memory, GPU, storage analysis
//! - **Platform Optimization**: OS-specific performance tuning
//! - **Usage Pattern Learning**: Adapts to your workflow patterns
//! - **Performance Classification**: Optimizes based on system capabilities
//!
//! ### 🌐 Ecosystem Discovery
//! - **Service Discovery**: Automatically finds Songbird, `BearDog`, `NestGate`, Squirrel
//! - **Network Scanning**: Scans local networks for available services
//! - **Health Monitoring**: Continuous service health assessment
//! - **Auto-Integration**: Seamless ecosystem service integration
//!
//! ### 🔧 Hardware Detection
//! - **CPU Analysis**: Cores, features, instruction sets, performance
//! - **Memory Profiling**: Capacity, type, availability assessment
//! - **GPU Discovery**: NVIDIA, AMD, Intel GPU detection and capabilities
//! - **Storage Classification**: SSD, HDD, `NVMe` detection and optimization
//!
//! ### 🎯 Zero-Touch Experience
//! - **No Configuration Required**: Sensible defaults for all scenarios
//! - **Automatic Optimization**: Performance tuning without user intervention
//! - **Self-Healing**: Adapts to hardware and network changes
//! - **Progressive Enhancement**: Works better as more services are available

pub mod ai_mcp_interface;
pub mod capability_traits;
pub mod ecosystem;
mod ecosystem_network;
mod ecosystem_types;
pub mod hardware;
pub mod installer;
pub mod intelligent;
pub mod natural_language;

// Re-export the main types for easy access
pub use ai_mcp_interface::{
    AiMcpInterface, AiPreferences, AiSession, ConfigurationSummary, ExecutionIntent, McpRequest,
    McpRequestType, McpResponse, PerformanceExpectations, ResourceHints, SessionInfo,
};
pub use capability_traits::{EcosystemServiceDiscoverer, HardwareCapabilityDetector};

/// Mock implementations for testing only. Production uses real `HardwareDetector` and
/// `EcosystemDiscoverer`. Evolution: Mocks avoid I/O in tests; real impls use sysinfo + network scan.
#[cfg(any(test, feature = "test-mocks"))]
pub use capability_traits::{MockEcosystemDiscoverer, MockHardwareDetector};
pub use ecosystem::{DiscoveredServices, EcosystemDiscoverer, ServiceInfo, ServiceType};
pub use hardware::{
    CpuInfo, GpuInfo, HardwareDetector, MemoryInfo, PerformanceClass, StorageInfo, StorageType,
    SystemCapabilities,
};
pub use installer::{ConfigManager, InstallationConfig, InstallationResult, SmartInstaller};
pub use intelligent::{
    ConfigSnapshot, IntelligentAutoConfig, PlatformConfig, PlatformOptimizer, UsageHints,
    UsageLearner,
};
pub use natural_language::{
    ConfigurationIntent, ConfigurationTemplate, ExplicitPreferences, IntentAnalysis,
    NaturalLanguageConfig, PerformancePreference, ResourcePreferences, RuntimePreferences,
    SecurityPreference, UsagePattern,
};

// Common result types
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;

/// Errors that can occur during auto-configuration
#[derive(Debug, thiserror::Error)]
pub enum ToadStoolError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Hardware detection error: {0}")]
    Hardware(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Ecosystem discovery error: {0}")]
    EcosystemDiscovery(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("External HTTP not supported - use Songbird for external HTTP")]
    ExternalHttpNotSupported,

    #[error("Other error: {0}")]
    Other(String),
}

impl ToadStoolError {
    /// Create a configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration(message.into())
    }

    /// Create a hardware detection error
    pub fn hardware<S: Into<String>>(message: S) -> Self {
        Self::Hardware(message.into())
    }

    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network(message.into())
    }

    /// Create an ecosystem discovery error
    pub fn ecosystem_discovery<S: Into<String>>(message: S) -> Self {
        Self::EcosystemDiscovery(message.into())
    }

    /// Create an other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}

/// Quick start function for zero-touch configuration
///
/// This is the simplest way to get `ToadStool` configured and running.
/// It performs all auto-configuration steps and returns a ready-to-use configuration.
///
/// # Examples
///
/// ```rust,ignore
/// // Example usage (API may change)
/// use toadstool_auto_config::quick_start;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = quick_start().await?;
///     println!("ToadStool is ready!");
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - Hardware detection fails
/// - System capabilities cannot be determined
/// - Configuration validation fails
/// - File system permissions prevent writing configuration files
pub async fn quick_start() -> ToadStoolResult<toadstool_config::ToadStoolConfig> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok(); // Ignore if already initialized

    tracing::info!("🍄 ToadStool Universal Compute Platform");
    tracing::info!("🎯 Zero-Touch Auto-Configuration Starting...");

    IntelligentAutoConfig::auto_configure().await
}

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
pub struct ConfigBuilder {
    enable_hardware_detection: bool,
    enable_ecosystem_discovery: bool,
    enable_performance_optimization: bool,
    enable_usage_learning: bool,
    discovery_timeout: std::time::Duration,
}

impl ConfigBuilder {
    /// Create a new configuration builder with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            enable_hardware_detection: true,
            enable_ecosystem_discovery: true,
            enable_performance_optimization: true,
            enable_usage_learning: true,
            discovery_timeout: std::time::Duration::from_secs(30),
        }
    }

    /// Enable or disable hardware detection
    #[must_use]
    pub fn with_hardware_detection(mut self, enable: bool) -> Self {
        self.enable_hardware_detection = enable;
        self
    }

    /// Enable or disable ecosystem service discovery
    #[must_use]
    pub fn with_ecosystem_discovery(mut self, enable: bool) -> Self {
        self.enable_ecosystem_discovery = enable;
        self
    }

    /// Enable or disable performance optimization
    #[must_use]
    pub fn with_performance_optimization(mut self, enable: bool) -> Self {
        self.enable_performance_optimization = enable;
        self
    }

    /// Enable or disable usage pattern learning
    #[must_use]
    pub fn with_usage_learning(mut self, enable: bool) -> Self {
        self.enable_usage_learning = enable;
        self
    }

    /// Set the discovery timeout
    #[must_use]
    pub fn with_discovery_timeout(mut self, timeout: std::time::Duration) -> Self {
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

/// System information summary for display and debugging
#[derive(Debug, Clone)]
pub struct SystemSummary {
    pub cpu_info: String,
    pub memory_info: String,
    pub gpu_info: String,
    pub storage_info: String,
    pub ecosystem_services: Vec<String>,
    pub performance_class: String,
    pub optimal_runtimes: Vec<String>,
}

impl SystemSummary {
    /// Create a system summary from detected capabilities
    #[must_use]
    pub fn from_capabilities(
        capabilities: &SystemCapabilities,
        ecosystem: &DiscoveredServices,
    ) -> Self {
        Self {
            cpu_info: format!(
                "{} ({} cores)",
                capabilities.cpu_info.model_name, capabilities.cpu_cores
            ),
            memory_info: format!("{:.1} GB", capabilities.memory_gb),
            gpu_info: if capabilities.gpu_count > 0 {
                format!("{} GPU(s)", capabilities.gpu_count)
            } else {
                "Integrated Graphics".to_string()
            },
            storage_info: format!(
                "{:.1} GB {:?}",
                capabilities.storage_gb, capabilities.storage_info.storage_type
            ),
            ecosystem_services: ecosystem.discovered_services.keys().cloned().collect(),
            performance_class: format!("{:?}", capabilities.performance_class),
            optimal_runtimes: vec!["Native".to_string()], // Would be determined by configuration
        }
    }

    /// Pretty print the system summary
    pub fn display(&self) {
        println!("🖥️  System Summary:");
        println!("   CPU: {}", self.cpu_info);
        println!("   Memory: {}", self.memory_info);
        println!("   GPU: {}", self.gpu_info);
        println!("   Storage: {}", self.storage_info);
        println!("   Performance: {}", self.performance_class);
        println!(
            "   Ecosystem Services: {}",
            if self.ecosystem_services.is_empty() {
                "None".to_string()
            } else {
                self.ecosystem_services.join(", ")
            }
        );
        println!("   Optimal Runtimes: {}", self.optimal_runtimes.join(", "));
    }
}

/// Get a human-readable system summary
///
/// This function performs basic hardware detection and ecosystem discovery
/// to provide a summary of the system capabilities and available services.
///
/// # Examples
///
/// ```rust,no_run
/// use toadstool_auto_config::get_system_summary;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let summary = get_system_summary().await?;
///     summary.display();
///     Ok(())
/// }
/// ```
pub async fn get_system_summary() -> ToadStoolResult<SystemSummary> {
    let mut hardware_detector = HardwareDetector::new();
    let mut ecosystem_discoverer = EcosystemDiscoverer::new();

    // Run hardware detection and ecosystem discovery sequentially (both need &mut self)
    let capabilities = hardware_detector.scan_system().await?;
    let ecosystem = ecosystem_discoverer.discover_services().await?;

    Ok(SystemSummary::from_capabilities(&capabilities, &ecosystem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "slow integration test - runs full system detection"]
    async fn test_quick_start() {
        // Test that quick_start doesn't panic
        let result = quick_start().await;

        // Should either succeed or fail gracefully
        match result {
            Ok(config) => {
                // Config should have some sensible defaults
                assert!(config.runtime.max_concurrent_executions > 0);
            }
            Err(e) => {
                // Errors should be informative
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_quick_start_sync() {
        // Fast synchronous test of default config generation
        let config = toadstool_config::ToadStoolConfig::default();

        // Verify default config has sensible values
        assert!(config.runtime.max_concurrent_executions > 0);
        assert!(config.runtime.resource_limits.max_cpu_usage > 0.0);
        assert!(config.runtime.resource_limits.max_memory_usage > 0.0);
    }

    #[test]
    fn test_config_builder_creation() {
        let builder = ConfigBuilder::new();

        assert!(builder.enable_hardware_detection);
        assert!(builder.enable_ecosystem_discovery);
        assert!(builder.enable_performance_optimization);
        assert!(builder.enable_usage_learning);
    }

    #[test]
    fn test_config_builder_customization() {
        let builder = ConfigBuilder::new()
            .with_hardware_detection(false)
            .with_ecosystem_discovery(false)
            .with_performance_optimization(true)
            .with_usage_learning(true);

        assert!(!builder.enable_hardware_detection);
        assert!(!builder.enable_ecosystem_discovery);
        assert!(builder.enable_performance_optimization);
        assert!(builder.enable_usage_learning);
    }

    #[test]
    fn test_config_builder_discovery_timeout() {
        let timeout = std::time::Duration::from_secs(10);
        let builder = ConfigBuilder::new().with_discovery_timeout(timeout);
        assert_eq!(builder.discovery_timeout, timeout);
    }

    #[test]
    fn test_config_builder_default() {
        let builder = ConfigBuilder::default();
        assert!(builder.enable_hardware_detection);
        assert_eq!(
            builder.discovery_timeout,
            std::time::Duration::from_secs(30)
        );
    }

    #[test]
    fn test_error_creation() {
        let config_error = ToadStoolError::configuration("test config error");
        assert!(config_error.to_string().contains("Configuration error"));

        let hardware_error = ToadStoolError::hardware("test hardware error");
        assert!(hardware_error
            .to_string()
            .contains("Hardware detection error"));

        let network_error = ToadStoolError::network("test network error");
        assert!(network_error.to_string().contains("Network error"));

        let ecosystem_error = ToadStoolError::ecosystem_discovery("discovery failed");
        assert!(ecosystem_error
            .to_string()
            .contains("Ecosystem discovery error"));

        let other_error = ToadStoolError::other("misc error");
        assert!(other_error.to_string().contains("Other error"));
    }

    #[test]
    fn test_external_http_error() {
        let err = ToadStoolError::ExternalHttpNotSupported;
        assert!(err.to_string().contains("External HTTP"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_system_summary_creation() {
        let capabilities = SystemCapabilities::default();
        let ecosystem = DiscoveredServices {
            discovered_services: std::collections::HashMap::new(),
            discovery_summary: ecosystem::DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };

        let summary = SystemSummary::from_capabilities(&capabilities, &ecosystem);

        assert!(!summary.cpu_info.is_empty());
        assert!(!summary.memory_info.is_empty());
        assert!(!summary.performance_class.is_empty());
    }

    #[test]
    fn test_system_summary_display() {
        let summary = SystemSummary {
            cpu_info: "Test CPU (4 cores)".to_string(),
            memory_info: "8.0 GB".to_string(),
            gpu_info: "Integrated Graphics".to_string(),
            storage_info: "100.0 GB SSD".to_string(),
            ecosystem_services: vec!["songbird".to_string()],
            performance_class: "Mainstream".to_string(),
            optimal_runtimes: vec!["Native".to_string()],
        };

        // Test that display doesn't panic
        summary.display();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder_build_with_all_disabled() {
        let builder = ConfigBuilder::new()
            .with_hardware_detection(false)
            .with_ecosystem_discovery(false)
            .with_performance_optimization(false)
            .with_usage_learning(false);
        let result = builder.build().await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.runtime.max_concurrent_executions > 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder_build_with_ecosystem_disabled() {
        let builder = ConfigBuilder::new().with_ecosystem_discovery(false);
        let result = builder.build().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_system_summary() {
        let result = get_system_summary().await;
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert!(!summary.cpu_info.is_empty());
        assert!(!summary.memory_info.is_empty());
        assert!(!summary.performance_class.is_empty());
    }

    #[test]
    fn test_system_summary_from_capabilities_with_ecosystem_services() {
        let mut discovered = std::collections::HashMap::new();
        discovered.insert(
            "songbird".to_string(),
            ServiceInfo {
                name: "songbird".to_string(),
                endpoint: "http://localhost:8080".to_string(),
                service_type: "NetworkCoordination".to_string(),
                version: "1.0".to_string(),
                capabilities: vec![],
                status: ecosystem::ServiceStatus::Healthy,
                discovered_via: "test".to_string(),
                response_time_ms: 0,
            },
        );
        let ecosystem = DiscoveredServices {
            discovered_services: discovered,
            discovery_summary: ecosystem::DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };
        let capabilities = SystemCapabilities::default();
        let summary = SystemSummary::from_capabilities(&capabilities, &ecosystem);
        assert_eq!(summary.ecosystem_services.len(), 1);
        assert!(summary.ecosystem_services.contains(&"songbird".to_string()));
    }
}
