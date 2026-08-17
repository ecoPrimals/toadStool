// SPDX-License-Identifier: AGPL-3.0-or-later
// Trait re-exported from compat module; async methods use RPITIT on `CompatibilityLayer`.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use crate::os_layer::compat::{
    LegacyCompatibilityLayer, LinuxCompatibilityLayer, MacOSCompatibilityLayer,
    WindowsCompatibilityLayer,
};
use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult, UniversalJob};
use tracing::info;

// Re-export the canonical `CompatibilityLayer` trait and dispatch enum from compat.
pub use super::compat::{CompatibilityLayer, CompatibilityLayerDispatch};

/// OS Layer Manager for universal compatibility
pub struct OSLayerManager {
    /// Available compatibility layers
    // Arc-wrapped so a layer can be selected under the lock and executed after
    // releasing it; the dispatch type is not Clone and executing while holding
    // the guard makes the caller !Send.
    compatibility_layers: Arc<RwLock<HashMap<String, Arc<CompatibilityLayerDispatch>>>>,
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

impl OSLayerManager {
    /// Creates a new OS layer manager with the given config.
    #[must_use]
    pub fn new(config: OSLayerConfig) -> Self {
        Self {
            config,
            compatibility_layers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the OS layer manager
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        info!("Initializing OS layer manager");

        let mut layers = self
            .compatibility_layers
            .write()
            .unwrap_or_else(|e| e.into_inner());

        // Initialize Linux compatibility layer
        if self.config.enabled {
            let linux_layer = LinuxCompatibilityLayer::new();
            layers.insert(
                "linux".to_string(),
                Arc::new(CompatibilityLayerDispatch::Linux(linux_layer)),
            );
        }

        // Initialize Windows compatibility layer
        if self.config.enabled {
            let windows_layer = WindowsCompatibilityLayer::new();
            layers.insert(
                "windows".to_string(),
                Arc::new(CompatibilityLayerDispatch::Windows(windows_layer)),
            );
        }

        // Initialize macOS compatibility layer
        if self.config.enabled {
            let macos_layer = MacOSCompatibilityLayer::new();
            layers.insert(
                "macos".to_string(),
                Arc::new(CompatibilityLayerDispatch::MacOS(macos_layer)),
            );
        }

        // Initialize legacy compatibility layer
        if self.config.enabled {
            let legacy_layer = LegacyCompatibilityLayer::new();
            layers.insert(
                "legacy".to_string(),
                Arc::new(CompatibilityLayerDispatch::Legacy(legacy_layer)),
            );
        }
        drop(layers);

        Ok(())
    }

    /// Execute a job with OS layer compatibility
    ///
    /// # Errors
    ///
    /// Returns error if the selected compatibility layer fails to execute the request.
    pub async fn execute_with_os_layer(
        &self,
        _job: &UniversalJob,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        // Select under the lock, execute after releasing it. Awaiting inside
        // the iteration held the read guard for the whole execution, blocking
        // layer registration and making this future !Send.
        let selected = {
            let layers = self
                .compatibility_layers
                .read()
                .unwrap_or_else(|e| e.into_inner());
            layers
                .iter()
                .find(|(_, layer)| layer.can_handle(&request))
                .map(|(name, layer)| (name.clone(), Arc::clone(layer)))
        };

        if let Some((name, layer)) = selected {
            info!("Using compatibility layer: {}", name);
            return layer.execute_with_compatibility(request).await;
        }

        Err(crate::ToadStoolError::not_supported(
            "No compatibility layer can handle this request. \
             Use capability-based execution dispatch via compute.execute instead.",
        ))
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
        let manager = OSLayerManager::new(config);
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

    #[tokio::test]
    async fn test_initialize_registers_layers_when_enabled() {
        let config = OSLayerConfig::default();
        let manager = OSLayerManager::new(config);
        manager.initialize().await.unwrap();
        let layers = manager
            .compatibility_layers
            .read()
            .unwrap_or_else(|e| e.into_inner());
        assert!(layers.contains_key("linux"));
        assert!(layers.contains_key("windows"));
        assert!(layers.contains_key("macos"));
        assert!(layers.contains_key("legacy"));
    }

    #[tokio::test]
    async fn test_initialize_disabled_registers_nothing() {
        let config = OSLayerConfig {
            enabled: false,
            ..OSLayerConfig::default()
        };
        let manager = OSLayerManager::new(config);
        manager.initialize().await.unwrap();
        let layers = manager
            .compatibility_layers
            .read()
            .unwrap_or_else(|e| e.into_inner());
        assert!(layers.is_empty());
    }

    fn test_job() -> UniversalJob {
        use crate::universal::types::{NetworkLocation, PrimalContext, SecurityLevel};
        use std::time::SystemTime;
        UniversalJob {
            id: uuid::Uuid::nil(),
            job_type: crate::UniversalJobType::Native {
                executable: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
            },
            priority: crate::universal::jobs::JobPriority::Normal,
            resources: crate::resources::ResourceRequirements::default(),
            timeout: None,
            created_at: SystemTime::now(),
            context: PrimalContext {
                user_id: "test".to_string(),
                device_id: "test".to_string(),
                session_id: "test".to_string(),
                network_location: NetworkLocation {
                    ip_address: "127.0.0.1".to_string(),
                    subnet: None,
                    network_id: None,
                    geo_location: None,
                },
                security_level: SecurityLevel::Standard,
                metadata: std::collections::HashMap::new(),
            },
        }
    }

    #[tokio::test]
    async fn test_execute_with_no_matching_layer_returns_error() {
        let config = OSLayerConfig {
            enabled: false,
            ..OSLayerConfig::default()
        };
        let manager = OSLayerManager::new(config);
        manager.initialize().await.unwrap();

        let request = ExecutionRequest::default();
        let result = manager.execute_with_os_layer(&test_job(), request).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No compatibility layer"));
    }

    #[tokio::test]
    async fn test_execute_routes_through_matching_layer() {
        let manager = OSLayerManager::new(OSLayerConfig::default());
        manager.initialize().await.unwrap();

        let request = ExecutionRequest::default();
        let result = manager.execute_with_os_layer(&test_job(), request).await;
        // A layer handled it (didn't fall through to the "No compatibility layer" error)
        if let Err(e) = &result {
            let msg = e.to_string();
            assert!(
                !msg.contains("No compatibility layer"),
                "expected a layer to handle the request, got fallthrough: {msg}"
            );
        }
    }
}
