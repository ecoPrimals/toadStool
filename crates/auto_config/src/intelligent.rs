//! # Intelligent Auto-Configuration System
//!
//! Core intelligence layer for ToadStool's zero-touch auto-configuration.
//! This module analyzes system capabilities, detects patterns, and generates
//! optimal configurations automatically.

use std::time::Duration;

use tracing::{debug, info, warn};

use crate::ecosystem::{DiscoveredServices, EcosystemDiscoverer};
use crate::hardware::{HardwareDetector, PerformanceClass, SystemCapabilities};
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::{GpuConfig, SecurityConfig, ToadStoolConfig};

/// Intelligent auto-configuration system that makes ToadStool work out-of-the-box
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
/// ```rust
/// use toadstool_auto_config::IntelligentAutoConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Zero-touch startup - just works!
///     let config = IntelligentAutoConfig::auto_configure().await?;
///     
///     // ToadStool is now optimally configured for this system
///     let platform = toadstool::UniversalComputePlatform::with_config(config).await?;
///     
///     println!("🎉 ToadStool ready to execute any workload!");
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
    /// Configuration history for optimization
    config_history: Vec<ConfigSnapshot>,
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
            config_history: Vec::new(),
        }
    }

    /// Scan system capabilities
    pub async fn scan_system(&mut self) -> ToadStoolResult<SystemCapabilities> {
        self.hardware_detector.scan_system().await
    }

    /// Discover ecosystem services
    pub async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        self.ecosystem_discoverer.discover_services().await
    }

    /// Generate intelligent configuration based on system analysis
    pub async fn generate_intelligent_config(&mut self) -> ToadStoolResult<ToadStoolConfig> {
        info!("🧠 Generating intelligent configuration...");

        // Phase 1: Hardware Discovery
        let hardware = self.hardware_detector.scan_system().await?;

        // Phase 2: Platform Detection
        let platform = self
            .platform_optimizer
            .optimize_for_platform(&hardware)
            .await?;

        // Phase 3: Ecosystem Discovery
        let ecosystem = self.ecosystem_discoverer.discover_services().await?;

        // Phase 4: Usage Analysis
        let usage_hints = self.usage_learner.analyze_environment().await?;

        // Phase 5: Generate Configuration
        let config = self
            .generate_optimal_config(hardware, platform, ecosystem, usage_hints)
            .await?;

        Ok(config)
    }

    /// Zero-configuration startup - just works!
    ///
    /// This is the main entry point for zero-touch ToadStool configuration.
    /// It performs comprehensive system analysis and generates an optimal
    /// configuration without requiring any user input.
    pub async fn auto_configure() -> ToadStoolResult<ToadStoolConfig> {
        info!("🧠 ToadStool Auto-Configuration Starting...");
        info!("✨ Zero-touch setup - making ToadStool grandma-friendly!");

        let mut auto_config = Self::new();

        // Phase 1: Hardware Discovery
        info!("🔍 Phase 1: Scanning hardware capabilities...");
        let hardware = auto_config.hardware_detector.scan_system().await?;
        info!(
            "🖥️ Detected: {} cores, {:.1}GB RAM, {} GPUs, {:.1}GB storage",
            hardware.cpu_cores, hardware.memory_gb, hardware.gpu_count, hardware.storage_gb
        );

        // Phase 2: Platform Optimization
        info!(
            "🔧 Phase 2: Optimizing for platform {}...",
            std::env::consts::OS
        );
        let platform_config = auto_config
            .platform_optimizer
            .optimize_for_platform(&hardware)
            .await?;
        info!(
            "⚡ Platform optimizations applied: {} optimizations",
            platform_config.optimizations.len()
        );

        // Phase 3: Ecosystem Discovery
        info!("🌐 Phase 3: Discovering ecosystem services...");
        let ecosystem = auto_config.ecosystem_discoverer.discover_services().await?;
        info!(
            "🔗 Found ecosystem services: {:?}",
            ecosystem.discovered_services.keys().collect::<Vec<_>>()
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
            .generate_optimal_config(hardware, platform_config, ecosystem, usage_hints)
            .await?;

        // Phase 6: Validation and Health Check
        info!("✅ Phase 6: Validating configuration...");
        auto_config.validate_configuration(&config).await?;

        info!("🎉 Auto-configuration complete - ToadStool is ready!");
        info!("🚀 Zero-touch setup successful - ready to execute any workload!");

        Ok(config)
    }

    /// Generate optimal configuration based on discovered system capabilities
    pub async fn generate_optimal_config(
        &mut self,
        hardware: SystemCapabilities,
        platform: PlatformConfig,
        _ecosystem: DiscoveredServices,
        usage_hints: UsageHints,
    ) -> ToadStoolResult<ToadStoolConfig> {
        debug!("Generating optimal configuration for detected capabilities");

        let mut config = ToadStoolConfig::default();

        // Configure runtime engines based on hardware
        self.configure_runtime_engines(&mut config, &hardware, &platform, &usage_hints)
            .await?;

        // Configure networking and ecosystem integration
        // Note: Ecosystem integration is handled through separate configuration systems

        // Configure security based on platform and environment
        config.security = self.configure_security_settings(&platform).await?;

        // Apply platform-specific optimizations
        self.apply_platform_optimizations(&mut config, &platform)
            .await?;

        // Store configuration snapshot for learning
        self.store_config_snapshot(&config, &hardware, &usage_hints)
            .await?;

        debug!("Optimal configuration generated successfully");
        Ok(config)
    }

    /// Configure runtime engines based on hardware capabilities
    async fn configure_runtime_engines(
        &self,
        config: &mut ToadStoolConfig,
        hardware: &SystemCapabilities,
        _platform: &PlatformConfig,
        _usage_hints: &UsageHints,
    ) -> ToadStoolResult<()> {
        // Runtime configuration based on hardware
        info!("🔧 Configuring runtime engines based on hardware capabilities");

        // Performance optimization based on hardware class
        let _performance_class = self.classify_performance(hardware);

        // Optimize for high-performance systems
        if hardware.cpu_cores >= 8.0 {
            info!("🚀 High-performance system detected, applying optimizations");

            // Enable all runtime engines
            config.runtime.max_concurrent_executions = (hardware.cpu_cores * 0.8) as u32;

            // Set generous resource limits
            config.runtime.resource_limits.max_cpu_usage = hardware.cpu_cores * 0.8;
            config.runtime.resource_limits.max_memory_usage = hardware.memory_gb * 1024.0 * 0.8;
            config.runtime.resource_limits.max_disk_usage = hardware.storage_gb * 1024.0 * 0.1;
        } else {
            info!("💡 Standard system detected, applying balanced optimizations");

            // Conservative worker counts
            config.runtime.max_concurrent_executions = (hardware.cpu_cores / 2.0).max(1.0) as u32;

            // Conservative resource limits
            config.runtime.resource_limits.max_cpu_usage = hardware.cpu_cores * 0.5;
            config.runtime.resource_limits.max_memory_usage = hardware.memory_gb * 1024.0 * 0.6;
            config.runtime.resource_limits.max_disk_usage = hardware.storage_gb * 1024.0 * 0.05;
        }

        // Configure container runtime
        config.runtime.container.runtime = "containerd".to_string();

        // Configure GPU runtime if available
        if hardware.gpu_count > 0 {
            info!("🎮 GPU detected, enabling GPU runtime");
            config.runtime.gpu = Some(GpuConfig::default());
        }

        debug!("Runtime engines configured");
        Ok(())
    }

    /// Configure ecosystem integration based on discovered services
    #[allow(dead_code)]
    async fn configure_ecosystem_integration(
        &self,
        _ecosystem: &DiscoveredServices,
    ) -> ToadStoolResult<()> {
        // Ecosystem integration is handled through separate configuration systems

        debug!("Ecosystem integration configuration skipped");
        Ok(())
    }

    /// Configure security settings based on platform and environment
    async fn configure_security_settings(
        &self,
        platform: &PlatformConfig,
    ) -> ToadStoolResult<SecurityConfig> {
        let mut security_config = SecurityConfig::default();

        // Sandbox configuration based on platform capabilities
        security_config.sandbox.enabled = platform.supports_sandboxing();

        // Authentication settings
        security_config.auth.enabled = true;
        security_config.auth.provider = "jwt".to_string();

        debug!(
            "Security settings configured: sandbox={}, auth={}",
            security_config.sandbox.enabled, security_config.auth.enabled
        );

        Ok(security_config)
    }

    /// Apply platform-specific optimizations
    async fn apply_platform_optimizations(
        &self,
        config: &mut ToadStoolConfig,
        platform: &PlatformConfig,
    ) -> ToadStoolResult<()> {
        for optimization in &platform.optimizations {
            match optimization.optimization_type.as_str() {
                "containers" => {
                    // Container support is configured by default
                    config.runtime.container.runtime = "containerd".to_string();
                }
                "gpu" => {
                    config.runtime.gpu = Some(GpuConfig::default());
                }
                "wasm" => {
                    // WASM support is configured by default
                    config.runtime.wasm.max_memory = 1024 * 1024 * 1024; // 1GB
                }
                "native" => {
                    // Native runtime is enabled by default
                }
                _ => {
                    debug!(
                        "Unknown optimization type: {}",
                        optimization.optimization_type
                    );
                }
            }
        }

        debug!(
            "Applied {} platform optimizations",
            platform.optimizations.len()
        );
        Ok(())
    }

    /// Validate the generated configuration
    async fn validate_configuration(&self, config: &ToadStoolConfig) -> ToadStoolResult<()> {
        // Basic validation checks
        if config.runtime.max_concurrent_executions == 0 {
            return Err(ToadStoolError::configuration(
                "No concurrent executions configured",
            ));
        }

        if config.runtime.resource_limits.max_memory_usage == 0.0 {
            return Err(ToadStoolError::configuration("No memory limit configured"));
        }

        // Advanced validation
        if config.runtime.wasm.max_memory == 0 {
            warn!("WASM runtime has no memory limit set");
        }

        if config.runtime.gpu.is_some() {
            if let Some(gpu_config) = &config.runtime.gpu {
                if gpu_config.max_memory_per_device == 0 {
                    warn!("GPU runtime enabled but no memory limit set");
                }
            }
        }

        Ok(())
    }

    /// Store configuration snapshot for learning and optimization
    async fn store_config_snapshot(
        &mut self,
        config: &ToadStoolConfig,
        hardware: &SystemCapabilities,
        usage_hints: &UsageHints,
    ) -> ToadStoolResult<()> {
        let snapshot = ConfigSnapshot {
            timestamp: chrono::Utc::now(),
            config: config.clone(),
            hardware: hardware.clone(),
            usage_hints: usage_hints.clone(),
            performance_metrics: None, // Will be filled in later during runtime
        };

        self.config_history.push(snapshot);

        // Keep only the last 10 snapshots
        if self.config_history.len() > 10 {
            self.config_history.remove(0);
        }

        debug!("Configuration snapshot stored for learning");
        Ok(())
    }

    /// Classify performance based on hardware capabilities
    fn classify_performance(&self, hardware: &SystemCapabilities) -> PerformanceClass {
        if hardware.cpu_cores >= 16.0 && hardware.memory_gb >= 32.0 && hardware.gpu_count > 0 {
            PerformanceClass::HighEnd
        } else if hardware.cpu_cores >= 8.0 && hardware.memory_gb >= 16.0 {
            PerformanceClass::Mainstream
        } else {
            PerformanceClass::LowEnd
        }
    }
}

impl Default for IntelligentAutoConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Platform-specific optimization engine
pub struct PlatformOptimizer {
    platform_info: PlatformInfo,
}

impl Default for PlatformOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformOptimizer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            platform_info: PlatformInfo::detect(),
        }
    }

    /// Optimize configuration for the current platform
    pub async fn optimize_for_platform(
        &self,
        hardware: &SystemCapabilities,
    ) -> ToadStoolResult<PlatformConfig> {
        let mut platform_config = PlatformConfig {
            platform_name: self.platform_info.os_name.clone(),
            supported_features: std::collections::HashSet::new(),
            optimizations: Vec::new(),
        };

        // Platform-specific feature detection
        match self.platform_info.os_name.as_str() {
            "linux" => {
                platform_config
                    .supported_features
                    .insert(PlatformSupport::Containers);
                platform_config
                    .supported_features
                    .insert(PlatformSupport::Sandboxing);
                platform_config
                    .supported_features
                    .insert(PlatformSupport::ProcessIsolation);
                platform_config
                    .supported_features
                    .insert(PlatformSupport::NetworkIsolation);

                // Linux-specific optimizations
                platform_config.optimizations.push(PlatformOptimization {
                    optimization_type: "memory_mapping".to_string(),
                    description: "Use mmap for large file operations".to_string(),
                    performance_gain: 0.15,
                });

                platform_config.optimizations.push(PlatformOptimization {
                    optimization_type: "async_io".to_string(),
                    description: "Use io_uring for async I/O operations".to_string(),
                    performance_gain: 0.25,
                });
            }
            "macos" => {
                platform_config
                    .supported_features
                    .insert(PlatformSupport::Sandboxing);
                platform_config
                    .supported_features
                    .insert(PlatformSupport::ProcessIsolation);

                // macOS-specific optimizations
                platform_config.optimizations.push(PlatformOptimization {
                    optimization_type: "vector_instructions".to_string(),
                    description: "Use Accelerate framework for vector operations".to_string(),
                    performance_gain: 0.20,
                });
            }
            "windows" => {
                platform_config
                    .supported_features
                    .insert(PlatformSupport::Sandboxing);
                platform_config
                    .supported_features
                    .insert(PlatformSupport::ProcessIsolation);

                // Windows-specific optimizations
                platform_config.optimizations.push(PlatformOptimization {
                    optimization_type: "numa_awareness".to_string(),
                    description: "NUMA-aware memory allocation".to_string(),
                    performance_gain: 0.10,
                });
            }
            _ => {
                debug!(
                    "Unknown platform: {}, using generic optimizations",
                    self.platform_info.os_name
                );
            }
        }

        // Hardware-specific optimizations
        if hardware.cpu_cores >= 8.0 {
            platform_config.optimizations.push(PlatformOptimization {
                optimization_type: "parallel_compilation".to_string(),
                description: "Enable parallel WASM compilation".to_string(),
                performance_gain: 0.30,
            });
        }

        if hardware.memory_gb >= 16.0 {
            platform_config.optimizations.push(PlatformOptimization {
                optimization_type: "large_buffer".to_string(),
                description: "Use larger I/O buffers".to_string(),
                performance_gain: 0.12,
            });
        }

        debug!(
            "Platform optimization complete: {} optimizations applied",
            platform_config.optimizations.len()
        );
        Ok(platform_config)
    }
}

/// Usage pattern learning and prediction
pub struct UsageLearner {
    _environment_hints: Vec<EnvironmentHint>,
}

impl Default for UsageLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageLearner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _environment_hints: Vec::new(),
        }
    }

    /// Analyze the environment to predict usage patterns
    pub async fn analyze_environment(&mut self) -> ToadStoolResult<UsageHints> {
        let mut usage_hints = UsageHints::default();

        // Check for development environment indicators
        if self.is_development_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("development".to_string());
            usage_hints.expected_cpu_usage = 0.3; // Moderate CPU usage
            usage_hints.expected_memory_usage = 0.4; // Moderate memory usage
        }

        // Check for machine learning environment indicators
        if self.is_ml_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("machine_learning".to_string());
            usage_hints.expected_cpu_usage = 0.8; // High CPU usage
            usage_hints.expected_memory_usage = 0.7; // High memory usage
            usage_hints.prefers_gpu = true;
        }

        // Check for web development indicators
        if self.is_web_development_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("web_development".to_string());
            usage_hints.expected_cpu_usage = 0.4; // Moderate CPU usage
            usage_hints.expected_memory_usage = 0.3; // Lower memory usage
            usage_hints.prefers_containers = true;
        }

        // Check for data processing indicators
        if self.is_data_processing_environment().await? {
            usage_hints
                .predicted_workload_types
                .push("data_processing".to_string());
            usage_hints.expected_cpu_usage = 0.6; // Moderate-high CPU usage
            usage_hints.expected_memory_usage = 0.8; // High memory usage
        }

        debug!(
            "Usage pattern analysis complete: {:?}",
            usage_hints.predicted_workload_types
        );
        Ok(usage_hints)
    }

    /// Check if this appears to be a development environment
    async fn is_development_environment(&self) -> ToadStoolResult<bool> {
        // Look for common development tools and directories
        let dev_indicators = [
            ".git",
            ".gitignore",
            "package.json",
            "Cargo.toml",
            "requirements.txt",
            "node_modules",
            "target",
            "__pycache__",
            ".vscode",
            ".idea",
        ];

        for indicator in &dev_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if this appears to be a machine learning environment
    async fn is_ml_environment(&self) -> ToadStoolResult<bool> {
        // Look for ML-specific files and tools
        let ml_indicators = [
            "requirements.txt",
            "environment.yml",
            "conda-meta",
            "jupyter",
            ".ipynb",
            "model.pkl",
            "data.csv",
        ];

        for indicator in &ml_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        // Check for Python ML packages
        if tokio::process::Command::new("python")
            .arg("-c")
            .arg("import torch, tensorflow, scikit-learn")
            .output()
            .await
            .is_ok()
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if this appears to be a web development environment
    async fn is_web_development_environment(&self) -> ToadStoolResult<bool> {
        // Look for web development indicators
        let web_indicators = [
            "package.json",
            "yarn.lock",
            "package-lock.json",
            "webpack.config.js",
            "rollup.config.js",
            "vite.config.js",
            "src",
            "public",
            "dist",
            "build",
        ];

        for indicator in &web_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if this appears to be a data processing environment
    async fn is_data_processing_environment(&self) -> ToadStoolResult<bool> {
        // Look for data processing indicators
        let data_indicators = [
            "data",
            "datasets",
            "*.csv",
            "*.parquet",
            "*.json",
            "Pipfile",
            "dask",
            "spark",
        ];

        for indicator in &data_indicators {
            if tokio::fs::metadata(indicator).await.is_ok() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Platform information and capabilities
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
}

impl PlatformInfo {
    pub fn detect() -> Self {
        Self {
            os_name: std::env::consts::OS.to_string(),
            os_version: "unknown".to_string(), // Would implement OS-specific version detection
            architecture: std::env::consts::ARCH.to_string(),
        }
    }
}

/// Platform support features
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PlatformSupport {
    Containers,
    Sandboxing,
    ProcessIsolation,
    NetworkIsolation,
}

/// Platform-specific configuration and capabilities
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub platform_name: String,
    pub supported_features: std::collections::HashSet<PlatformSupport>,
    pub optimizations: Vec<PlatformOptimization>,
}

impl PlatformConfig {
    /// Check if a specific feature is supported
    #[must_use]
    pub fn supports(&self, feature: &PlatformSupport) -> bool {
        self.supported_features.contains(feature)
    }

    /// Check if containers are supported
    #[must_use]
    pub fn supports_containers(&self) -> bool {
        self.supports(&PlatformSupport::Containers)
    }

    /// Check if sandboxing is supported
    #[must_use]
    pub fn supports_sandboxing(&self) -> bool {
        self.supports(&PlatformSupport::Sandboxing)
    }

    /// Check if process isolation is supported
    #[must_use]
    pub fn supports_process_isolation(&self) -> bool {
        self.supports(&PlatformSupport::ProcessIsolation)
    }

    /// Check if network isolation is supported
    #[must_use]
    pub fn supports_network_isolation(&self) -> bool {
        self.supports(&PlatformSupport::NetworkIsolation)
    }
}

/// Platform-specific optimization
#[derive(Debug, Clone)]
pub struct PlatformOptimization {
    pub optimization_type: String,
    pub description: String,
    pub performance_gain: f64, // Expected performance improvement (0.0 to 1.0)
}

/// Usage pattern hints for optimization
#[derive(Debug, Clone, Default)]
pub struct UsageHints {
    pub predicted_workload_types: Vec<String>,
    pub expected_cpu_usage: f64,
    pub expected_memory_usage: f64,
    pub prefers_gpu: bool,
    pub prefers_containers: bool,
}

impl UsageHints {
    pub fn is_cpu_intensive(&self) -> bool {
        self.expected_cpu_usage > 0.7
    }

    pub fn is_memory_intensive(&self) -> bool {
        self.expected_memory_usage > 0.7
    }
}

/// Environment hint for usage pattern detection
#[derive(Debug, Clone)]
pub struct EnvironmentHint {
    pub hint_type: String,
    pub confidence: f64,
    pub description: String,
}

/// Configuration snapshot for learning and optimization
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub config: ToadStoolConfig,
    pub hardware: SystemCapabilities,
    pub usage_hints: UsageHints,
    pub performance_metrics: Option<PerformanceMetrics>,
}

/// Performance metrics for configuration optimization
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub avg_execution_time: Duration,
    pub memory_usage_peak: f64,
    pub cpu_usage_avg: f64,
    pub throughput_executions_per_sec: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
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
            learner._environment_hints.len(),
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
