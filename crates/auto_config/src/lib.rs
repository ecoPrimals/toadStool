// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::missing_errors_doc
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
//! - **🤖 AI-Native**: Perfect integration with intelligence service AI interface
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
//! - **Service Discovery**: Automatically finds coordination, security, storage, and intelligence services
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
/// `EcosystemDiscoverer`. Evolution: Mocks avoid I/O in tests; real impls use toadstool-sysmon + network scan.
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

mod bootstrap;
mod config_builder;
mod error;
mod system_summary;

pub use bootstrap::quick_start;
pub use config_builder::ConfigBuilder;
pub use error::{ToadStoolError, ToadStoolResult};
pub use system_summary::{SystemSummary, get_system_summary};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "slow integration test - runs full system detection"
    )]
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
        assert!(
            hardware_error
                .to_string()
                .contains("Hardware detection error")
        );

        let network_error = ToadStoolError::network("test network error");
        assert!(network_error.to_string().contains("Network error"));

        let ecosystem_error = ToadStoolError::ecosystem_discovery("discovery failed");
        assert!(
            ecosystem_error
                .to_string()
                .contains("Ecosystem discovery error")
        );

        let other_error = ToadStoolError::other("misc error");
        assert!(other_error.to_string().contains("Other error"));
    }

    #[test]
    fn test_external_http_error() {
        let err = ToadStoolError::ExternalHttpNotSupported;
        assert!(err.to_string().contains("External HTTP"));
    }

    #[test]
    fn test_toadstool_error_io_and_json_display() {
        let io_err: ToadStoolError = std::io::Error::other("disk full").into();
        let io_msg = io_err.to_string();
        assert!(io_msg.contains("IO error"), "got: {io_msg}");
        assert!(io_msg.contains("disk full"), "got: {io_msg}");

        let json_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
        let json_e: ToadStoolError = json_err.into();
        let json_msg = json_e.to_string();
        assert!(json_msg.contains("JSON"), "got: {json_msg}");
    }

    #[test]
    fn test_config_builder_default_matches_new() {
        let d = ConfigBuilder::default();
        let n = ConfigBuilder::new();
        assert_eq!(d.enable_hardware_detection, n.enable_hardware_detection);
        assert_eq!(d.enable_ecosystem_discovery, n.enable_ecosystem_discovery);
        assert_eq!(
            d.enable_performance_optimization,
            n.enable_performance_optimization
        );
        assert_eq!(d.enable_usage_learning, n.enable_usage_learning);
        assert_eq!(d.discovery_timeout, n.discovery_timeout);
        assert!(d.enable_hardware_detection);
        assert!(d.enable_ecosystem_discovery);
        assert!(d.enable_performance_optimization);
        assert!(d.enable_usage_learning);
        assert_eq!(d.discovery_timeout, std::time::Duration::from_secs(30));
    }

    /// Chains `with_*` toggles (fine-grained “custom” build) and checks final flag state.
    #[test]
    fn test_config_builder_chained_with_methods() {
        let b = ConfigBuilder::new()
            .with_hardware_detection(true)
            .with_ecosystem_discovery(false)
            .with_performance_optimization(true)
            .with_usage_learning(false)
            .with_discovery_timeout(std::time::Duration::from_secs(7));
        assert!(b.enable_hardware_detection);
        assert!(!b.enable_ecosystem_discovery);
        assert!(b.enable_performance_optimization);
        assert!(!b.enable_usage_learning);
        assert_eq!(b.discovery_timeout, std::time::Duration::from_secs(7));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder_build_with_hardware_detection_disabled() {
        let builder = ConfigBuilder::new().with_hardware_detection(false);
        let result = builder.build().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder_build_with_usage_learning_disabled() {
        let builder = ConfigBuilder::new().with_usage_learning(false);
        let result = builder.build().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder_build_with_performance_optimization_disabled() {
        let builder = ConfigBuilder::new().with_performance_optimization(false);
        let result = builder.build().await;
        assert!(result.is_ok());
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
            ecosystem_services: vec!["coordination".to_string()],
            performance_class: "Mainstream".to_string(),
            optimal_runtimes: vec!["Native".to_string()],
        };

        // Test that display doesn't panic
        summary.display();
    }

    #[test]
    fn test_system_summary_display_empty_ecosystem_services() {
        let summary = SystemSummary {
            cpu_info: "CPU".to_string(),
            memory_info: "1.0 GB".to_string(),
            gpu_info: "Integrated Graphics".to_string(),
            storage_info: "10.0 GB".to_string(),
            ecosystem_services: vec![],
            performance_class: "LowEnd".to_string(),
            optimal_runtimes: vec!["Native".to_string()],
        };
        summary.display();
    }

    #[test]
    fn test_system_summary_from_capabilities_gpu_branch() {
        let capabilities = SystemCapabilities {
            gpu_count: 2,
            ..SystemCapabilities::default()
        };
        let ecosystem = DiscoveredServices {
            discovered_services: std::collections::HashMap::new(),
            discovery_summary: ecosystem::DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };
        let summary = SystemSummary::from_capabilities(&capabilities, &ecosystem);
        assert!(
            summary.gpu_info.contains("2 GPU"),
            "got: {}",
            summary.gpu_info
        );
    }

    #[test]
    fn test_system_summary_from_capabilities_formatting() {
        let mut capabilities = SystemCapabilities::default();
        capabilities.cpu_info.model_name = "TestModel".to_string();
        capabilities.cpu_cores = 8.0;
        capabilities.memory_gb = 16.0;
        capabilities.performance_class = PerformanceClass::HighEnd;
        let ecosystem = DiscoveredServices {
            discovered_services: std::collections::HashMap::new(),
            discovery_summary: ecosystem::DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };
        let summary = SystemSummary::from_capabilities(&capabilities, &ecosystem);
        assert!(summary.cpu_info.contains("TestModel"));
        assert!(summary.cpu_info.contains('8'));
        assert!(summary.memory_info.contains("16.0"));
        assert!(summary.performance_class.contains("HighEnd"));
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

    /// Basic smoke: `get_system_summary` returns a summary usable for display.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_system_summary_basic_call() {
        let summary = get_system_summary().await.expect("get_system_summary");
        summary.display();
        assert!(!summary.gpu_info.is_empty());
        assert!(!summary.storage_info.is_empty());
    }

    #[test]
    fn test_system_summary_from_capabilities_with_ecosystem_services() {
        let mut discovered = std::collections::HashMap::new();
        let test_endpoint = format!(
            "{}{}:{}",
            toadstool_common::constants::network::HTTP_PROTOCOL,
            toadstool_common::constants::network::LOCALHOST_IPV4,
            8080
        );
        discovered.insert(
            "coordination".to_string(),
            ServiceInfo {
                name: "coordination".to_string(),
                endpoint: test_endpoint,
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
        assert!(
            summary
                .ecosystem_services
                .contains(&"coordination".to_string())
        );
    }
}
