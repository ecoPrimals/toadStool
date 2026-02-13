//! Model loaders for various formats
//!
//! Supports loading models from HuggingFace Hub and local files.

pub mod safetensors;

use crate::Result;
use std::path::Path;

/// Load model weights from a directory
pub fn load_weights<P: AsRef<Path>>(path: P) -> Result<ModelWeights> {
    let path = path.as_ref();

    // Check for safetensors (preferred)
    let safetensors_path = path.join("model.safetensors");
    if safetensors_path.exists() {
        return safetensors::load(&safetensors_path);
    }

    // Check for pytorch bin
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
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get data as f32 slice (for F32 tensors)
    pub fn as_f32(&self) -> Option<&[f32]> {
        if matches!(self.dtype, DataType::F32) {
            Some(bytemuck::cast_slice(&self.data))
        } else {
            None
        }
    }
}
