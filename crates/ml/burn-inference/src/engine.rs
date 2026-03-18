// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inference engine for running models
//!
//! Provides a high-level API for model loading and device selection.
//! For actual inference, use model-specific APIs: `Bert::forward()`, `Whisper::transcribe()`,
//! `Yolo::detect()`, `ResNet::classify()` — each returns a clear error if weights/backend are missing.

use crate::device::enumerate_devices;
use crate::{BurnDevice, Error, Result};
use std::path::Path;
use tracing::info;

/// Configuration for the inference engine
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Enable mixed precision (fp16 where beneficial)
    pub mixed_precision: bool,
    /// Enable tensor fusion optimizations
    pub tensor_fusion: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            mixed_precision: true,
            tensor_fusion: true,
        }
    }
}

/// Main inference engine
pub struct InferenceEngine {
    device: BurnDevice,
    #[allow(dead_code, reason = "stored for future batch/precision/fusion overrides")]
    config: EngineConfig,
}

impl InferenceEngine {
    /// Create a new inference engine with auto-selected device
    #[must_use]
    pub fn new() -> Self {
        Self::with_device(BurnDevice::auto_select())
    }

    /// Create with specific device
    pub fn with_device(device: BurnDevice) -> Self {
        let info = device.info();
        info!(
            "InferenceEngine initialized on: {} ({:?})",
            info.name, info.device_type
        );

        Self {
            device,
            config: EngineConfig::default(),
        }
    }

    /// Create with device and config
    #[must_use]
    pub const fn with_config(device: BurnDevice, config: EngineConfig) -> Self {
        Self { device, config }
    }

    /// Get the underlying device
    #[must_use]
    pub const fn device(&self) -> &BurnDevice {
        &self.device
    }

    /// Load a model from a file
    pub fn load_model<P: AsRef<Path>>(&self, path: P) -> Result<LoadedModel> {
        let path = path.as_ref();
        info!("Loading model from: {}", path.display());

        // Detect model format from extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "bin" | "safetensors" => {
                // HuggingFace format
                info!("Detected HuggingFace model format");
                Ok(LoadedModel {
                    name: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    format: ModelFormat::HuggingFace,
                    size_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                })
            }
            "onnx" => {
                info!("Detected ONNX model format");
                Ok(LoadedModel {
                    name: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    format: ModelFormat::Onnx,
                    size_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                })
            }
            _ => Err(Error::UnsupportedModel(format!("Unknown format: {ext}"))),
        }
    }

    /// Run inference
    ///
    /// Generic inference is not implemented. Use model-specific APIs with loaded weights.
    ///
    /// # Errors
    /// Returns `Error::ModelBackendRequired` with available backends and model-specific API guidance.
    pub fn infer(&self, model: &LoadedModel, _input: &[f32]) -> Result<Vec<f32>> {
        let backends: Vec<String> = enumerate_devices()
            .iter()
            .map(|d| format!("{} ({:?})", d.name, d.device_type))
            .collect();
        Err(Error::ModelBackendRequired(format!(
            "Generic inference not supported for '{}' ({:?}). \
             Available backends: [{}]. \
             Use model-specific APIs with loaded weights: Bert::forward(), Whisper::transcribe(), \
             Yolo::detect(), or ResNet::classify(). Each returns a clear error if weights are missing.",
            model.name,
            model.format,
            backends.join(", ")
        )))
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// A loaded model ready for inference
#[derive(Debug)]
pub struct LoadedModel {
    pub name: String,
    pub format: ModelFormat,
    pub size_bytes: u64,
}

/// Supported model formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// `HuggingFace` safetensors/bin format
    HuggingFace,
    /// ONNX format
    Onnx,
    /// Burn native format
    BurnNative,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = InferenceEngine::new();
        let info = engine.device().info();
        println!("Engine device: {} ({:?})", info.name, info.device_type);
    }

    #[test]
    fn test_engine_config() {
        let config = EngineConfig {
            max_batch_size: 64,
            mixed_precision: false,
            tensor_fusion: true,
        };

        let engine = InferenceEngine::with_config(BurnDevice::cpu(), config);
        assert!(!engine.device().is_gpu());
    }
}
