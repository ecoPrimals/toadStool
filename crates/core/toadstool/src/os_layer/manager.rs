// SPDX-License-Identifier: AGPL-3.0-or-later
// async_trait is no longer needed since we re-export the trait from compat module
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::os_layer::compat::{
    LegacyCompatibilityLayer, LinuxCompatibilityLayer, MacOSCompatibilityLayer,
    WindowsCompatibilityLayer,
};
use crate::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeMetrics,
    RuntimeType, ToadStoolResult, UniversalJob,
};
use tracing::info;

/// OS Layer Manager for universal compatibility
pub struct OSLayerManager {
    /// Available compatibility layers
    compatibility_layers: Arc<RwLock<HashMap<String, Box<dyn CompatibilityLayer>>>>,
    /// OS layer configuration
    config: OSLayerConfig,
}

/// Configuration for OS layer functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSLayerConfig {
    /// Enable OS layer compatibility
    pub enabled: bool,
    /// Available compatibility modes
    pub available_modes: Vec<String>,
    /// Default compatibility mode
    pub default_mode: String,
    /// Maximum nesting depth for OS layers
    pub max_nesting_depth: u32,
}

impl Default for OSLayerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            available_modes: vec![
                "linux".to_string(),
                "windows".to_string(),
                "macos".to_string(),
                "freebsd".to_string(),
                "openbsd".to_string(),
                "netbsd".to_string(),
                "solaris".to_string(),
                "aix".to_string(),
                "hpux".to_string(),
                "legacy".to_string(),
            ],
            default_mode: std::env::consts::OS.to_string(),
            max_nesting_depth: 5,
        }
    }
}

/// Platform information for the current system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system name
    pub os: String,
    /// Architecture
    pub arch: String,
    /// OS version
    pub version: String,
    /// Kernel version
    pub kernel: String,
    /// Available features
    pub features: Vec<String>,
}

impl PlatformInfo {
    /// Detect current platform information
    #[must_use]
    pub fn detect() -> Self {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let version = "unknown".to_string(); // Could be enhanced with actual version detection
        let kernel = "unknown".to_string();

        let features = vec![
            #[cfg(unix)]
            "unix".to_string(),
            #[cfg(windows)]
            "windows".to_string(),
            #[cfg(target_os = "linux")]
            "linux".to_string(),
            #[cfg(target_os = "macos")]
            "macos".to_string(),
            #[cfg(target_os = "freebsd")]
            "freebsd".to_string(),
        ];

        Self {
            os,
            arch,
            version,
            kernel,
            features,
        }
    }
}

// Re-export the canonical CompatibilityLayer trait from compat module
// The complete trait definition is in `compat.rs` with 5 methods.
// This provides backward compatibility for any code importing from here.
pub use super::compat::CompatibilityLayer;

impl OSLayerManager {
    #[must_use]
    pub fn new(config: OSLayerConfig) -> Self {
        Self {
            config,
            compatibility_layers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the OS layer manager
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        info!("Initializing OS layer manager");

        let mut layers = self.compatibility_layers.write().await;

        // Initialize Linux compatibility layer
        if self.config.enabled {
            let linux_layer = LinuxCompatibilityLayer::new();
            layers.insert("linux".to_string(), Box::new(linux_layer));
        }

        // Initialize Windows compatibility layer
        if self.config.enabled {
            let windows_layer = WindowsCompatibilityLayer::new();
            layers.insert("windows".to_string(), Box::new(windows_layer));
        }

        // Initialize macOS compatibility layer
        if self.config.enabled {
            let macos_layer = MacOSCompatibilityLayer::new();
            layers.insert("macos".to_string(), Box::new(macos_layer));
        }

        // Initialize legacy compatibility layer
        if self.config.enabled {
            let legacy_layer = LegacyCompatibilityLayer::new();
            layers.insert("legacy".to_string(), Box::new(legacy_layer));
        }
        drop(layers);

        Ok(())
    }

    /// Execute a job with OS layer compatibility
    pub async fn execute_with_os_layer(
        &self,
        _job: &UniversalJob,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        // Try to find a suitable compatibility layer
        for (name, layer) in self.compatibility_layers.read().await.iter() {
            if layer.can_handle(&request) {
                info!("Using compatibility layer: {}", name);
                return layer.execute_with_compatibility(request).await;
            }
        }

        // Fallback to default execution
        Ok(ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                stdout: Some("Default OS layer execution".to_string()),
                ..Default::default()
            },
            metrics: RuntimeMetrics::default(),
            duration: Duration::from_millis(100),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        })
    }

    /// Get platform information
    #[must_use]
    pub fn get_platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: "unknown".to_string(),
            kernel: "unknown".to_string(),
            features: vec![std::env::consts::FAMILY.to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_layer_config_default() {
        let config = OSLayerConfig::default();
        assert!(config.enabled);
        assert!(!config.available_modes.is_empty());
        assert!(config.available_modes.contains(&"linux".to_string()));
        assert!(config.available_modes.contains(&"windows".to_string()));
        assert!(config.available_modes.contains(&"macos".to_string()));
        assert_eq!(config.max_nesting_depth, 5);
    }

    #[test]
    fn test_os_layer_config_default_mode() {
        let config = OSLayerConfig::default();
        assert_eq!(config.default_mode, std::env::consts::OS);
    }

    #[test]
    fn test_os_layer_config_serialization_roundtrip() {
        let config = OSLayerConfig {
            enabled: false,
            available_modes: vec!["test".to_string()],
            default_mode: "custom".to_string(),
            max_nesting_depth: 10,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: OSLayerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.available_modes, deserialized.available_modes);
        assert_eq!(config.default_mode, deserialized.default_mode);
        assert_eq!(config.max_nesting_depth, deserialized.max_nesting_depth);
    }

    #[test]
    fn test_platform_info_detect() {
        let info = PlatformInfo::detect();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert_eq!(info.version, "unknown");
        assert_eq!(info.kernel, "unknown");
        assert!(!info.features.is_empty());
    }

    #[test]
    fn test_platform_info_serialization_roundtrip() {
        let info = PlatformInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            version: "1.0".to_string(),
            kernel: "5.0".to_string(),
            features: vec!["unix".to_string()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: PlatformInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info.os, deserialized.os);
        assert_eq!(info.arch, deserialized.arch);
        assert_eq!(info.features, deserialized.features);
    }

    #[test]
    fn test_os_layer_manager_new() {
        let config = OSLayerConfig::default();
        let manager = OSLayerManager::new(config.clone());
        let info = manager.get_platform_info();
        assert_eq!(info.os, std::env::consts::OS);
        assert_eq!(info.arch, std::env::consts::ARCH);
        assert_eq!(info.features, vec![std::env::consts::FAMILY.to_string()]);
    }

    #[test]
    fn test_os_layer_manager_custom_config() {
        let config = OSLayerConfig {
            enabled: false,
            available_modes: vec!["custom".to_string()],
            default_mode: "custom".to_string(),
            max_nesting_depth: 1,
        };
        let manager = OSLayerManager::new(config);
        let info = manager.get_platform_info();
        assert!(!info.os.is_empty());
    }
}
