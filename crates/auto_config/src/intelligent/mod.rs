// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Intelligent Auto-Configuration System
//!
//! Core intelligence layer for `ToadStool`'s zero-touch auto-configuration.
//! This module analyzes system capabilities, detects patterns, and generates
//! optimal configurations automatically.
//!
//! ## Pipeline Architecture
//!
//! This module is organized into 4 pipeline stages:
//! - **detection**: Platform and capability detection (Stage 1)
//! - **analysis**: Pattern recognition and usage learning (Stage 2)
//! - **generation**: Configuration generation (Stage 3)
//! - **validation**: Configuration validation (Stage 4)

pub mod analysis;
pub mod detection;
pub mod generation;
pub mod validation;

// Re-export all public types for backward compatibility
pub use analysis::*;
pub use detection::*;
pub use generation::*;
pub use validation::*;

use tracing::info;

use crate::ToadStoolResult;
use crate::ecosystem::{DiscoveredServices, EcosystemDiscoverer};
use crate::hardware::{HardwareDetector, SystemCapabilities};
use toadstool_config::ToadStoolConfig;

/// Intelligent auto-configuration system that makes `ToadStool` work out-of-the-box
/// with optimal settings for any environment.
///
/// # Zero-Touch Philosophy
///
/// This system embodies the "zero-touch" principle:
/// - **Works immediately**: No configuration files needed
/// - **Optimizes automatically**: Detects best settings for hardware
/// - **Self-heals**: Adapts to changing conditions
/// - **Grandma-friendly**: So simple that anyone can use it
///
/// # Examples
///
/// ```rust,ignore
/// // Example usage (API may change)
/// use toadstool_auto_config::IntelligentAutoConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Zero-touch startup - just works!
///     let config = IntelligentAutoConfig::auto_configure().await?;
///     
///     // Configuration is now ready to use
///     println!("🎉 ToadStool auto-configured successfully!");
///     Ok(())
/// }
/// ```
pub struct IntelligentAutoConfig {
    /// Hardware detection and optimization
    pub hardware_detector: HardwareDetector,
    /// Platform-specific optimizations
    pub platform_optimizer: PlatformOptimizer,
    /// Network and ecosystem discovery
    pub ecosystem_discoverer: EcosystemDiscoverer,
    /// Usage pattern learning
    pub usage_learner: UsageLearner,
    /// Configuration generator
    pub config_generator: ConfigGenerator,
    /// Configuration validator
    pub config_validator: ConfigValidator,
}

impl IntelligentAutoConfig {
    /// Create a new intelligent auto-configuration system
    #[must_use]
    pub fn new() -> Self {
        Self {
            hardware_detector: HardwareDetector::new(),
            platform_optimizer: PlatformOptimizer::new(),
            ecosystem_discoverer: EcosystemDiscoverer::new(),
            usage_learner: UsageLearner::new(),
            config_generator: ConfigGenerator::new(),
            config_validator: ConfigValidator::new(),
        }
    }

    /// Scan system capabilities
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if hardware detection fails or system
    /// capabilities cannot be determined.
    #[must_use = "System scan result should be checked"]
    pub async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        self.hardware_detector.scan_system().await
    }

    /// Discover ecosystem services
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if service discovery fails or network
    /// scanning encounters errors.
    #[must_use = "Service discovery result should be checked"]
    pub async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        self.ecosystem_discoverer.discover_services().await
    }

    /// Generate intelligent configuration based on system analysis
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - Hardware scanning fails.
    /// - Platform optimization fails.
    /// - Service discovery fails.
    /// - Configuration generation or validation fails.
    #[must_use = "Configuration generation result should be checked"]
    pub async fn generate_intelligent_config(&mut self) -> ToadStoolResult<ToadStoolConfig> {
        info!("🧠 Generating intelligent configuration...");

        // ✅ CONCURRENT EXECUTION: Launch independent discovery phases in parallel
        // Only Platform Detection depends on Hardware Discovery - everything else is independent
        let (hardware_result, ecosystem_result, usage_result) = tokio::join!(
            // Phase 1: Hardware Discovery
            self.hardware_detector.scan_system(),
            // Phase 3: Ecosystem Discovery (independent, can run concurrently)
            self.ecosystem_discoverer.discover_services(),
            // Phase 4: Usage Analysis (independent, can run concurrently)
            self.usage_learner.analyze_environment(),
        );

        let hardware = hardware_result?;
        let ecosystem = ecosystem_result?;
        let usage_hints = usage_result?;

        // Phase 2: Platform Detection (depends on hardware, must be sequential)
        let platform = self.platform_optimizer.optimize_for_platform(&hardware)?;

        // Phase 5: Generate Configuration
        let config = self
            .config_generator
            .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
            .await?;

        Ok(config)
    }

    /// Zero-configuration startup - just works!
    ///
    /// This is the main entry point for zero-touch `ToadStool` configuration.
    /// It performs comprehensive system analysis and generates an optimal
    /// configuration without requiring any user input.
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - Hardware scanning fails during system detection.
    /// - Platform optimization encounters errors.
    /// - Ecosystem discovery fails.
    /// - Configuration validation fails.
    /// - Any phase of the auto-configuration process encounters an error.
    #[must_use = "Auto-configuration result should be checked"]
    pub async fn auto_configure() -> ToadStoolResult<ToadStoolConfig> {
        info!("🧠 ToadStool Auto-Configuration Starting...");
        info!("✨ Zero-touch setup - making ToadStool grandma-friendly!");

        let mut auto_config = Self::new();

        // ✅ CONCURRENT EXECUTION: Launch independent discovery phases in parallel
        info!("🔍 Phases 1/3/4: Scanning hardware, ecosystem, and usage patterns concurrently...");
        let (hardware_result, ecosystem_result, usage_result) = tokio::join!(
            auto_config.hardware_detector.scan_system(),
            auto_config.ecosystem_discoverer.discover_services(),
            auto_config.usage_learner.analyze_environment(),
        );

        let hardware = hardware_result?;
        let ecosystem = ecosystem_result?;
        let _usage_hints = usage_result?;

        info!(
            "🖥️ Hardware: {} cores, {:.1}GB RAM, {} GPUs, {:.1}GB storage",
            hardware.cpu_cores, hardware.memory_gb, hardware.gpu_count, hardware.storage_gb
        );
        info!(
            "🔗 Ecosystem: {} services discovered",
            ecosystem.discovered_services.len()
        );

        // Phase 2: Platform Optimization (depends on hardware, must be sequential)
        info!(
            "🔧 Phase 2: Optimizing for platform {}...",
            std::env::consts::OS
        );
        let platform_config = auto_config
            .platform_optimizer
            .optimize_for_platform(&hardware)?;
        info!(
            "⚡ Platform optimizations applied: {} optimizations",
            platform_config.optimizations.len()
        );

        // Phase 4: Usage Pattern Analysis
        info!("📊 Phase 4: Analyzing usage patterns...");
        let usage_hints = auto_config.usage_learner.analyze_environment().await?;
        info!(
            "🎯 Usage patterns detected: {:?}",
            usage_hints.predicted_workload_types
        );

        // Phase 5: Generate Optimal Configuration
        info!("⚙️ Phase 5: Generating optimal configuration...");
        let config = auto_config
            .config_generator
            .generate_optimal_config(&hardware, &platform_config, &ecosystem, &usage_hints)
            .await?;

        // Phase 6: Validation and Health Check
        info!("✅ Phase 6: Validating configuration...");
        auto_config
            .config_validator
            .validate_configuration(&config)?;

        info!("🎉 Auto-configuration complete - ToadStool is ready!");
        info!("🚀 Zero-touch setup successful - ready to execute any workload!");

        Ok(config)
    }
}

impl Default for IntelligentAutoConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "slow integration test - runs hardware/network detection"
    )]
    async fn test_auto_configuration_basic() {
        let result = IntelligentAutoConfig::auto_configure().await;
        assert!(result.is_ok(), "Auto-configuration should succeed");

        let config = result.unwrap();
        assert!(
            config.runtime.max_concurrent_executions > 0,
            "Should have at least one concurrent execution"
        );
        assert!(
            config.runtime.resource_limits.max_cpu_usage > 0.0,
            "Should have positive CPU usage limit"
        );
    }

    #[test]
    fn test_intelligent_auto_config_creation() {
        // Fast synchronous test that doesn't require hardware scanning
        let _auto_config = IntelligentAutoConfig::new();

        // If it constructs without panicking, the test passes
        // (using _ to avoid unused variable warning)
    }

    #[test]
    fn test_platform_optimizer_creation() {
        let optimizer = PlatformOptimizer::new();
        assert!(
            !optimizer.platform_info.os_name.is_empty(),
            "Should detect OS name"
        );
    }

    #[test]
    fn test_usage_learner_creation() {
        let learner = UsageLearner::new();
        assert_eq!(
            learner.environment_hints.len(),
            0,
            "Should start with no hints"
        );
    }

    #[test]
    fn test_usage_hints_detection() {
        let hints = UsageHints {
            expected_cpu_usage: 0.8,
            expected_memory_usage: 0.6,
            ..Default::default()
        };

        assert!(
            hints.is_cpu_intensive(),
            "Should detect CPU intensive usage"
        );
        assert!(
            !hints.is_memory_intensive(),
            "Should not detect memory intensive usage"
        );
    }
}
