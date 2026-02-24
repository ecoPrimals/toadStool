use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toadstool::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

// Import from canonical core compat layer
use toadstool::os_layer::compat::{
    CompatibilityLayer, LinuxCompatibilityLayer, MacOSCompatibilityLayer, WindowsCompatibilityLayer,
};

/// OS layer manager for distributed execution
pub struct OSLayerManager {
    /// Configuration
    config: OSLayerConfig,
    /// Compatibility layers
    compatibility_layers: HashMap<String, CompatibilityLayerEnum>,
}

/// Configuration for OS layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSLayerConfig {
    /// Enable OS layer compatibility
    pub enabled: bool,
    /// Default OS layer
    pub default_layer: String,
    /// Available layers
    pub available_layers: Vec<String>,
}

impl Default for OSLayerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_layer: "linux".to_string(),
            available_layers: vec![
                "linux".to_string(),
                "windows".to_string(),
                "macos".to_string(),
            ],
        }
    }
}

/// Enum to hold different compatibility layer types
/// Now uses the canonical CompatibilityLayer trait from core
#[derive(Debug)]
pub enum CompatibilityLayerEnum {
    Linux(LinuxCompatibilityLayer),
    Windows(WindowsCompatibilityLayer),
    MacOS(MacOSCompatibilityLayer),
}

impl CompatibilityLayerEnum {
    pub async fn initialize(&mut self) -> ToadStoolResult<()> {
        match self {
            Self::Linux(layer) => CompatibilityLayer::initialize(layer).await,
            Self::Windows(layer) => CompatibilityLayer::initialize(layer).await,
            Self::MacOS(layer) => CompatibilityLayer::initialize(layer).await,
        }
    }

    pub async fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        match self {
            Self::Linux(layer) => {
                CompatibilityLayer::execute_with_compatibility(layer, request).await
            }
            Self::Windows(layer) => {
                CompatibilityLayer::execute_with_compatibility(layer, request).await
            }
            Self::MacOS(layer) => {
                CompatibilityLayer::execute_with_compatibility(layer, request).await
            }
        }
    }

    pub async fn shutdown(&mut self) -> ToadStoolResult<()> {
        match self {
            Self::Linux(layer) => CompatibilityLayer::shutdown(layer).await,
            Self::Windows(layer) => CompatibilityLayer::shutdown(layer).await,
            Self::MacOS(layer) => CompatibilityLayer::shutdown(layer).await,
        }
    }
}

impl OSLayerManager {
    #[must_use]
    pub fn new(config: OSLayerConfig) -> Self {
        Self {
            config,
            compatibility_layers: HashMap::new(),
        }
    }

    pub async fn initialize(&mut self) -> ToadStoolResult<()> {
        // Initialize compatibility layers using canonical core implementation
        if self.config.available_layers.contains(&"linux".to_string()) {
            let mut layer = CompatibilityLayerEnum::Linux(LinuxCompatibilityLayer::new());
            layer.initialize().await?;
            self.compatibility_layers.insert("linux".to_string(), layer);
        }

        if self
            .config
            .available_layers
            .contains(&"windows".to_string())
        {
            let mut layer = CompatibilityLayerEnum::Windows(WindowsCompatibilityLayer::new());
            layer.initialize().await?;
            self.compatibility_layers
                .insert("windows".to_string(), layer);
        }

        if self.config.available_layers.contains(&"macos".to_string()) {
            let mut layer = CompatibilityLayerEnum::MacOS(MacOSCompatibilityLayer::new());
            layer.initialize().await?;
            self.compatibility_layers.insert("macos".to_string(), layer);
        }

        Ok(())
    }

    pub async fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        let layer_name = &self.config.default_layer;

        if let Some(layer) = self.compatibility_layers.get(layer_name) {
            layer.execute_with_compatibility(request).await
        } else {
            Err(toadstool::ToadStoolError::configuration(format!(
                "Compatibility layer '{layer_name}' not found"
            )))
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
        assert_eq!(config.default_layer, "linux");
        assert!(config.available_layers.contains(&"linux".to_string()));
        assert!(config.available_layers.contains(&"windows".to_string()));
        assert!(config.available_layers.contains(&"macos".to_string()));
    }

    #[test]
    fn test_os_layer_config_custom() {
        let config = OSLayerConfig {
            enabled: false,
            default_layer: "custom".to_string(),
            available_layers: vec!["custom".to_string()],
        };
        assert!(!config.enabled);
        assert_eq!(config.default_layer, "custom");
        assert_eq!(config.available_layers.len(), 1);
    }

    #[test]
    fn test_os_layer_manager_creation() {
        let config = OSLayerConfig::default();
        let manager = OSLayerManager::new(config);
        assert!(std::mem::size_of_val(&manager) > 0);
    }

    #[test]
    fn test_os_layer_config_serialization() {
        let config = OSLayerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: OSLayerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default_layer, config.default_layer);
    }

    #[tokio::test]
    async fn test_os_layer_manager_initialize() {
        let config = OSLayerConfig {
            enabled: true,
            default_layer: "linux".to_string(),
            available_layers: vec!["linux".to_string()],
        };
        let mut manager = OSLayerManager::new(config);
        let result = manager.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_os_layer_manager_execute_with_compatibility() {
        let config = OSLayerConfig {
            enabled: true,
            default_layer: "linux".to_string(),
            available_layers: vec!["linux".to_string()],
        };
        let mut manager = OSLayerManager::new(config);
        manager.initialize().await.unwrap();

        let request = ExecutionRequest::default();

        let result = manager.execute_with_compatibility(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_os_layer_manager_layer_not_found() {
        let config = OSLayerConfig {
            enabled: true,
            default_layer: "nonexistent".to_string(),
            available_layers: vec!["linux".to_string()],
        };
        let manager = OSLayerManager::new(config);
        assert!(manager.compatibility_layers.is_empty());

        let request = ExecutionRequest::default();

        let result = manager.execute_with_compatibility(request).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not found"));
    }

    #[tokio::test]
    async fn test_compatibility_layer_enum_initialize() {
        let mut layer = CompatibilityLayerEnum::Linux(LinuxCompatibilityLayer::new());
        let result = layer.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compatibility_layer_enum_execute() {
        let mut layer = CompatibilityLayerEnum::Linux(LinuxCompatibilityLayer::new());
        layer.initialize().await.unwrap();

        let request = ExecutionRequest::default();

        let result = layer.execute_with_compatibility(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compatibility_layer_enum_shutdown() {
        let mut layer = CompatibilityLayerEnum::Linux(LinuxCompatibilityLayer::new());
        layer.initialize().await.unwrap();
        let result = layer.shutdown().await;
        assert!(result.is_ok());
    }
}
