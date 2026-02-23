//! Model loaders for various formats
//!
//! Supports loading models from `HuggingFace` Hub (safetensors) and llama.cpp (GGUF).
//!
//! ## Supported Formats
//!
//! - **Safetensors**: `HuggingFace` standard format, full precision (F32, F16, BF16)
//! - **GGUF**: llama.cpp format, supports quantized models (`Q4_0`, `Q8_0`, etc.)

pub mod gguf;
pub mod safetensors;

use crate::Result;
use std::path::Path;

/// Load model weights from a directory or file
///
/// Automatically detects format based on file extension or directory contents:
/// - `.gguf` files → GGUF loader
/// - `.safetensors` files → Safetensors loader
/// - Directory with `model.safetensors` → Safetensors loader
pub fn load_weights<P: AsRef<Path>>(path: P) -> Result<ModelWeights> {
    let path = path.as_ref();

    // Handle direct file paths
    if path.is_file() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        return match ext {
            "gguf" => gguf::load(path),
            "safetensors" => safetensors::load(path),
            _ => Err(crate::Error::ModelLoad(format!(
                "Unknown file extension: {ext} (expected .gguf or .safetensors)"
            ))),
        };
    }

    // Handle directories - prefer safetensors, then GGUF
    let safetensors_path = path.join("model.safetensors");
    if safetensors_path.exists() {
        return safetensors::load(&safetensors_path);
    }

    // Check for any safetensors file
    for entry in std::fs::read_dir(path).map_err(crate::Error::Io)? {
        let entry = entry.map_err(crate::Error::Io)?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|e| e.to_str()) == Some("safetensors") {
            return safetensors::load(&entry_path);
        }
    }

    // Check for GGUF files
    for entry in std::fs::read_dir(path).map_err(crate::Error::Io)? {
        let entry = entry.map_err(crate::Error::Io)?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|e| e.to_str()) == Some("gguf") {
            return gguf::load(&entry_path);
        }
    }

    // Check for pytorch bin (unsupported)
    let pytorch_path = path.join("pytorch_model.bin");
    if pytorch_path.exists() {
        tracing::warn!("pytorch_model.bin requires conversion - use safetensors format");
        return Err(crate::Error::UnsupportedModel(
            "pytorch_model.bin not yet supported, convert to safetensors".to_string(),
        ));
    }

    Err(crate::Error::ModelLoad(format!(
        "No supported weights found in {}",
        path.display()
    )))
}

/// Container for loaded model weights
#[derive(Debug)]
pub struct ModelWeights {
    /// Weight tensors by name
    pub tensors: std::collections::HashMap<String, WeightTensor>,
    /// Model metadata
    pub metadata: Option<serde_json::Value>,
}

/// A loaded weight tensor
#[derive(Debug)]
pub struct WeightTensor {
    /// Tensor name
    pub name: String,
    /// Shape
    pub shape: Vec<usize>,
    /// Data type
    pub dtype: DataType,
    /// Raw data bytes
    pub data: Vec<u8>,
}

/// Supported data types
#[derive(Debug, Clone, Copy)]
pub enum DataType {
    F32,
    F16,
    BF16,
    I32,
    I64,
}

impl WeightTensor {
    /// Get number of elements
    #[must_use]
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get data as f32 slice (for F32 tensors)
    #[must_use]
    pub fn as_f32(&self) -> Option<&[f32]> {
        if matches!(self.dtype, DataType::F32) {
            Some(bytemuck::cast_slice(&self.data))
        } else {
            None
        }
    }
}
