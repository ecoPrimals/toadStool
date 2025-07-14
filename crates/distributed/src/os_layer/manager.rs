use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toadstool::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

use crate::compatibility::layers::{
    LinuxCompatibilityLayer, MacOSCompatibilityLayer, WindowsCompatibilityLayer,
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
#[derive(Debug, Clone)]
pub enum CompatibilityLayerEnum {
    Linux(LinuxCompatibilityLayer),
    Windows(WindowsCompatibilityLayer),
    MacOS(MacOSCompatibilityLayer),
}

impl CompatibilityLayerEnum {
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        match self {
            CompatibilityLayerEnum::Linux(layer) => layer.initialize().await,
            CompatibilityLayerEnum::Windows(layer) => layer.initialize().await,
            CompatibilityLayerEnum::MacOS(layer) => layer.initialize().await,
        }
    }

    pub async fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        match self {
            CompatibilityLayerEnum::Linux(layer) => layer.execute_with_compatibility(request).await,
            CompatibilityLayerEnum::Windows(layer) => {
                layer.execute_with_compatibility(request).await
            }
            CompatibilityLayerEnum::MacOS(layer) => layer.execute_with_compatibility(request).await,
        }
    }

    pub async fn cleanup(&self) -> ToadStoolResult<()> {
        match self {
            CompatibilityLayerEnum::Linux(layer) => layer.cleanup().await,
            CompatibilityLayerEnum::Windows(layer) => layer.cleanup().await,
            CompatibilityLayerEnum::MacOS(layer) => layer.cleanup().await,
        }
    }
}

impl OSLayerManager {
    pub fn new(config: OSLayerConfig) -> Self {
        Self {
            config,
            compatibility_layers: HashMap::new(),
        }
    }

    pub async fn initialize(&mut self) -> ToadStoolResult<()> {
        // Initialize compatibility layers
        if self.config.available_layers.contains(&"linux".to_string()) {
            let layer = CompatibilityLayerEnum::Linux(LinuxCompatibilityLayer::new());
            layer.initialize().await?;
            self.compatibility_layers.insert("linux".to_string(), layer);
        }

        if self
            .config
            .available_layers
            .contains(&"windows".to_string())
        {
            let layer = CompatibilityLayerEnum::Windows(WindowsCompatibilityLayer::new());
            layer.initialize().await?;
            self.compatibility_layers
                .insert("windows".to_string(), layer);
        }

        if self.config.available_layers.contains(&"macos".to_string()) {
            let layer = CompatibilityLayerEnum::MacOS(MacOSCompatibilityLayer::new());
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
