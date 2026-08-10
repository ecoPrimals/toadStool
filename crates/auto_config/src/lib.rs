// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![expect(
    clippy::doc_markdown,
    reason = "technical identifiers pervasive in API docs"
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
mod ecosystem_types;
pub mod hardware;
pub mod intelligent;
pub mod installer;
pub mod natural_language;

#[cfg(feature = "runtime")]
mod ecosystem_network;

#[cfg(feature = "runtime")]
pub mod ecosystem;

mod error;
mod system_summary;

#[cfg(feature = "runtime")]
mod bootstrap;
#[cfg(feature = "runtime")]
mod config_builder;

// Re-export the main types for easy access
pub use ai_mcp_interface::{
    AiPreferences, AiSession, ConfigurationSummary, ExecutionIntent, McpRequest, McpRequestType,
    McpResponse, PerformanceExpectations, ResourceHints, SessionInfo,
};
#[cfg(feature = "runtime")]
pub use ai_mcp_interface::AiMcpInterface;
pub use capability_traits::{EcosystemServiceDiscoverer, HardwareCapabilityDetector};

/// Mock implementations for testing only. Production uses real `HardwareDetector` and
/// `EcosystemDiscoverer`. Evolution: Mocks avoid I/O in tests; real impls use toadstool-sysmon + network scan.
#[cfg(any(test, feature = "test-mocks"))]
pub use capability_traits::{MockEcosystemDiscoverer, MockHardwareDetector};
pub use ecosystem_types::{
    DiscoveredServices, DiscoverySummary, ServiceInfo, ServicePattern, ServiceStatus, ServiceType,
};
#[cfg(feature = "runtime")]
pub use ecosystem::EcosystemDiscoverer;
pub use hardware::{
    CpuInfo, GpuInfo, HardwareDetector, MemoryInfo, PerformanceClass, StorageInfo, StorageType,
    SystemCapabilities,
};
pub use installer::{InstallationConfig, InstallationResult};
#[cfg(feature = "runtime")]
pub use installer::{ConfigManager, SmartInstaller};
pub use intelligent::{
    ConfigSnapshot, PlatformConfig, PlatformOptimizer, UsageHints, UsageLearner,
};
#[cfg(feature = "runtime")]
pub use intelligent::IntelligentAutoConfig;
pub use natural_language::{
    ConfigurationIntent, ConfigurationTemplate, ExplicitPreferences, IntentAnalysis,
    PerformancePreference, ResourcePreferences, RuntimePreferences, SecurityPreference,
    UsagePattern,
};
#[cfg(feature = "runtime")]
pub use natural_language::NaturalLanguageConfig;

#[cfg(feature = "runtime")]
pub use bootstrap::quick_start;
#[cfg(feature = "runtime")]
pub use config_builder::ConfigBuilder;
pub use error::{ToadStoolError, ToadStoolResult};
pub use system_summary::SystemSummary;
#[cfg(feature = "runtime")]
pub use system_summary::get_system_summary;

#[cfg(all(test, feature = "runtime"))]
#[path = "lib_tests.rs"]
mod tests;
