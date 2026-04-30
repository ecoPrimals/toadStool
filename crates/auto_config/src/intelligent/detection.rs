// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform detection and capability discovery (Pipeline Stage 1)

use tracing::debug;

use crate::ToadStoolResult;
use crate::hardware::SystemCapabilities;

/// Platform-specific optimization engine.
pub struct PlatformOptimizer {
    /// Detected platform information.
    pub platform_info: PlatformInfo,
}

impl Default for PlatformOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformOptimizer {
    /// Creates a new platform optimizer with detected platform info.
    #[must_use]
    pub fn new() -> Self {
        Self {
            platform_info: PlatformInfo::detect(),
        }
    }

    /// Optimize configuration for the current platform
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// if platform-specific optimizations fail.
    #[must_use = "Platform optimization result should be checked"]
    pub fn optimize_for_platform(
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

/// Platform information and capabilities.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// OS name (e.g. linux, macos, windows).
    pub os_name: String,
    /// OS version string.
    pub os_version: String,
    /// CPU architecture (e.g. x86_64, aarch64).
    pub architecture: String,
}

impl PlatformInfo {
    /// Detects the current platform.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            os_name: std::env::consts::OS.to_string(),
            os_version: "unknown".to_string(), // Would implement OS-specific version detection
            architecture: std::env::consts::ARCH.to_string(),
        }
    }
}

/// Platform support features.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PlatformSupport {
    /// Container support (Docker, etc.).
    Containers,
    /// Sandboxing support.
    Sandboxing,
    /// Process isolation support.
    ProcessIsolation,
    /// Network isolation support.
    NetworkIsolation,
}

/// Platform-specific configuration and capabilities.
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// Platform name.
    pub platform_name: String,
    /// Supported platform features.
    pub supported_features: std::collections::HashSet<PlatformSupport>,
    /// Platform-specific optimizations.
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

/// Platform-specific optimization.
#[derive(Debug, Clone)]
pub struct PlatformOptimization {
    /// Optimization type identifier.
    pub optimization_type: String,
    /// Human-readable description.
    pub description: String,
    /// Expected performance improvement (0.0 to 1.0).
    pub performance_gain: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux_info() -> PlatformInfo {
        PlatformInfo {
            os_name: "linux".to_string(),
            os_version: "6.1".to_string(),
            architecture: "x86_64".to_string(),
        }
    }

    fn macos_info() -> PlatformInfo {
        PlatformInfo {
            os_name: "macos".to_string(),
            os_version: "14.0".to_string(),
            architecture: "aarch64".to_string(),
        }
    }

    fn windows_info() -> PlatformInfo {
        PlatformInfo {
            os_name: "windows".to_string(),
            os_version: "11".to_string(),
            architecture: "x86_64".to_string(),
        }
    }

    fn small_hw() -> SystemCapabilities {
        SystemCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            ..Default::default()
        }
    }

    fn large_hw() -> SystemCapabilities {
        SystemCapabilities {
            cpu_cores: 16.0,
            memory_gb: 64.0,
            ..Default::default()
        }
    }

    #[test]
    fn platform_info_detect_returns_valid() {
        let info = PlatformInfo::detect();
        assert!(!info.os_name.is_empty());
        assert!(!info.architecture.is_empty());
    }

    #[test]
    fn platform_optimizer_default() {
        let opt = PlatformOptimizer::default();
        assert!(!opt.platform_info.os_name.is_empty());
    }

    #[test]
    fn linux_platform_supports_containers() {
        let opt = PlatformOptimizer {
            platform_info: linux_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        assert!(config.supports_containers());
        assert!(config.supports_sandboxing());
        assert!(config.supports_process_isolation());
        assert!(config.supports_network_isolation());
    }

    #[test]
    fn linux_platform_optimizations() {
        let opt = PlatformOptimizer {
            platform_info: linux_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        let types: Vec<&str> = config
            .optimizations
            .iter()
            .map(|o| o.optimization_type.as_str())
            .collect();
        assert!(types.contains(&"memory_mapping"));
        assert!(types.contains(&"async_io"));
    }

    #[test]
    fn macos_platform_features() {
        let opt = PlatformOptimizer {
            platform_info: macos_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        assert!(!config.supports_containers());
        assert!(config.supports_sandboxing());
        assert!(config.supports_process_isolation());
        assert!(!config.supports_network_isolation());
    }

    #[test]
    fn macos_platform_optimizations() {
        let opt = PlatformOptimizer {
            platform_info: macos_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        let types: Vec<&str> = config
            .optimizations
            .iter()
            .map(|o| o.optimization_type.as_str())
            .collect();
        assert!(types.contains(&"vector_instructions"));
    }

    #[test]
    fn windows_platform_features() {
        let opt = PlatformOptimizer {
            platform_info: windows_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        assert!(!config.supports_containers());
        assert!(config.supports_sandboxing());
    }

    #[test]
    fn large_hardware_adds_parallel_and_buffer_opts() {
        let opt = PlatformOptimizer {
            platform_info: linux_info(),
        };
        let config = opt.optimize_for_platform(&large_hw()).unwrap();
        let types: Vec<&str> = config
            .optimizations
            .iter()
            .map(|o| o.optimization_type.as_str())
            .collect();
        assert!(types.contains(&"parallel_compilation"));
        assert!(types.contains(&"large_buffer"));
    }

    #[test]
    fn small_hardware_no_parallel_opt() {
        let opt = PlatformOptimizer {
            platform_info: linux_info(),
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        let types: Vec<&str> = config
            .optimizations
            .iter()
            .map(|o| o.optimization_type.as_str())
            .collect();
        assert!(!types.contains(&"parallel_compilation"));
        assert!(!types.contains(&"large_buffer"));
    }

    #[test]
    fn unknown_platform_generic_only() {
        let opt = PlatformOptimizer {
            platform_info: PlatformInfo {
                os_name: "freebsd".to_string(),
                os_version: "14".to_string(),
                architecture: "x86_64".to_string(),
            },
        };
        let config = opt.optimize_for_platform(&small_hw()).unwrap();
        assert!(config.supported_features.is_empty());
    }

    #[test]
    fn platform_config_supports_method() {
        let mut config = PlatformConfig {
            platform_name: "test".to_string(),
            supported_features: std::collections::HashSet::new(),
            optimizations: Vec::new(),
        };
        assert!(!config.supports(&PlatformSupport::Containers));
        config
            .supported_features
            .insert(PlatformSupport::Containers);
        assert!(config.supports(&PlatformSupport::Containers));
    }
}
