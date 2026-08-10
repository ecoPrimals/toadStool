// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration generation (Pipeline Stage 3)

use tracing::{debug, info};

use crate::ToadStoolResult;
use crate::ecosystem_types::DiscoveredServices;
use crate::hardware::SystemCapabilities;
use toadstool_config::{GpuConfig, SecurityConfig, ToadStoolConfig};

use super::analysis::{ConfigSnapshot, UsageHints, classify_performance};
use super::detection::PlatformConfig;

/// Configuration generator
pub struct ConfigGenerator {
    /// Configuration history for optimization
    config_history: Vec<ConfigSnapshot>,
}

impl Default for ConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigGenerator {
    /// Creates a new configuration generator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            config_history: Vec::new(),
        }
    }

    /// Generate optimal configuration based on discovered system capabilities
    #[expect(
        clippy::unused_async,
        reason = "API contract for future async operations"
    )]
    pub async fn generate_optimal_config(
        &mut self,
        hardware: &SystemCapabilities,
        platform: &PlatformConfig,
        _ecosystem: &DiscoveredServices,
        usage_hints: &UsageHints,
    ) -> ToadStoolResult<ToadStoolConfig> {
        debug!("Generating optimal configuration for detected capabilities");

        let mut config = ToadStoolConfig::default();

        // Configure runtime engines based on hardware
        self.configure_runtime_engines(&mut config, hardware, platform, usage_hints);

        // Configure networking and ecosystem integration
        // Note: Ecosystem integration is handled through separate configuration systems

        // Configure security based on platform and environment
        config.security = self.configure_security_settings(platform);

        // Apply platform-specific optimizations
        self.apply_platform_optimizations(&mut config, platform);

        // Store configuration snapshot for learning
        self.store_config_snapshot(&config, hardware, usage_hints);

        debug!("Optimal configuration generated successfully");
        Ok(config)
    }

    /// Configure runtime engines based on hardware capabilities
    #[expect(clippy::unused_self, reason = "may use self for future extensions")]
    fn configure_runtime_engines(
        &self,
        config: &mut ToadStoolConfig,
        hardware: &SystemCapabilities,
        _platform: &PlatformConfig,
        _usage_hints: &UsageHints,
    ) {
        // Runtime configuration based on hardware
        info!("🔧 Configuring runtime engines based on hardware capabilities");

        // Performance optimization based on hardware class
        let _performance_class = classify_performance(hardware);

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
    }

    /// Configure security settings based on platform and environment
    #[expect(clippy::unused_self, reason = "may use self for future extensions")]
    fn configure_security_settings(&self, platform: &PlatformConfig) -> SecurityConfig {
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

        security_config
    }

    /// Apply platform-specific optimizations
    #[expect(clippy::unused_self, reason = "may use self for future extensions")]
    fn apply_platform_optimizations(
        &self,
        config: &mut ToadStoolConfig,
        platform: &PlatformConfig,
    ) {
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
    }

    /// Store configuration snapshot for learning and optimization
    fn store_config_snapshot(
        &mut self,
        config: &ToadStoolConfig,
        hardware: &SystemCapabilities,
        usage_hints: &UsageHints,
    ) {
        let snapshot = ConfigSnapshot {
            timestamp: std::time::SystemTime::now(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_hw() -> SystemCapabilities {
        SystemCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            ..Default::default()
        }
    }

    fn large_hw() -> SystemCapabilities {
        SystemCapabilities {
            cpu_cores: 16.0,
            memory_gb: 64.0,
            storage_gb: 2000.0,
            gpu_count: 2,
            ..Default::default()
        }
    }

    fn linux_platform() -> PlatformConfig {
        PlatformConfig {
            platform_name: "linux".to_string(),
            supported_features: {
                let mut s = std::collections::HashSet::new();
                s.insert(super::super::detection::PlatformSupport::Sandboxing);
                s
            },
            optimizations: Vec::new(),
        }
    }

    fn empty_ecosystem() -> DiscoveredServices {
        DiscoveredServices {
            discovered_services: std::collections::HashMap::new(),
            discovery_summary: crate::ecosystem_types::DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn config_generator_default() {
        let cg = ConfigGenerator::default();
        assert!(cg.config_history.is_empty());
    }

    #[tokio::test]
    async fn generate_config_small_hardware() {
        let mut cg = ConfigGenerator::new();
        let hints = UsageHints::default();
        let config = cg
            .generate_optimal_config(&small_hw(), &linux_platform(), &empty_ecosystem(), &hints)
            .await
            .unwrap();
        assert!(config.runtime.max_concurrent_executions >= 1);
        assert!(config.runtime.gpu.is_none());
    }

    #[tokio::test]
    async fn generate_config_large_hardware_enables_gpu() {
        let mut cg = ConfigGenerator::new();
        let hints = UsageHints::default();
        let config = cg
            .generate_optimal_config(&large_hw(), &linux_platform(), &empty_ecosystem(), &hints)
            .await
            .unwrap();
        assert!(config.runtime.gpu.is_some());
        assert!(config.runtime.max_concurrent_executions > 4);
    }

    #[tokio::test]
    async fn security_settings_respect_platform_sandboxing() {
        let mut cg = ConfigGenerator::new();
        let hints = UsageHints::default();
        let config = cg
            .generate_optimal_config(&small_hw(), &linux_platform(), &empty_ecosystem(), &hints)
            .await
            .unwrap();
        assert!(config.security.sandbox.enabled);
        assert!(config.security.auth.enabled);
    }

    #[tokio::test]
    async fn stores_config_snapshots() {
        let mut cg = ConfigGenerator::new();
        let hints = UsageHints::default();
        for _ in 0..3 {
            cg.generate_optimal_config(&small_hw(), &linux_platform(), &empty_ecosystem(), &hints)
                .await
                .unwrap();
        }
        assert_eq!(cg.config_history.len(), 3);
    }

    #[tokio::test]
    async fn config_history_capped_at_10() {
        let mut cg = ConfigGenerator::new();
        let hints = UsageHints::default();
        for _ in 0..12 {
            cg.generate_optimal_config(&small_hw(), &linux_platform(), &empty_ecosystem(), &hints)
                .await
                .unwrap();
        }
        assert_eq!(cg.config_history.len(), 10);
    }

    #[test]
    fn apply_container_optimization() {
        let cg = ConfigGenerator::new();
        let mut config = ToadStoolConfig::default();
        let mut platform = linux_platform();
        platform
            .optimizations
            .push(super::super::detection::PlatformOptimization {
                optimization_type: "containers".to_string(),
                description: "test".to_string(),
                performance_gain: 0.1,
            });
        cg.apply_platform_optimizations(&mut config, &platform);
        assert_eq!(config.runtime.container.runtime, "containerd");
    }

    #[test]
    fn apply_wasm_optimization() {
        let cg = ConfigGenerator::new();
        let mut config = ToadStoolConfig::default();
        let mut platform = linux_platform();
        platform
            .optimizations
            .push(super::super::detection::PlatformOptimization {
                optimization_type: "wasm".to_string(),
                description: "test".to_string(),
                performance_gain: 0.1,
            });
        cg.apply_platform_optimizations(&mut config, &platform);
        assert_eq!(config.runtime.wasm.max_memory, 1024 * 1024 * 1024);
    }
}
