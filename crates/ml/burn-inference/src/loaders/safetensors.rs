//! Safetensors file loader
//!
//! Loads model weights from HuggingFace safetensors format.

use super::{DataType, ModelWeights, WeightTensor};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Load weights from a safetensors file
pub fn load<P: AsRef<Path>>(path: P) -> Result<ModelWeights> {
    let path = path.as_ref();
    tracing::info!("Loading safetensors from {}", path.display());

    let mut file = File::open(path).map_err(Error::Io)?;

    // Read header size (first 8 bytes, little-endian u64)
    let mut header_size_bytes = [0u8; 8];
    file.read_exact(&mut header_size_bytes)
        .map_err(|e| Error::ModelLoad(format!("Failed to read header size: {e}")))?;
    let header_size = u64::from_le_bytes(header_size_bytes) as usize;

    tracing::debug!("Safetensors header size: {header_size}");

    // Read header JSON
    let mut header_bytes = vec![0u8; header_size];
    file.read_exact(&mut header_bytes)
        .map_err(|e| Error::ModelLoad(format!("Failed to read header: {e}")))?;

    let header: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|e| Error::ModelLoad(format!("Invalid header JSON: {e}")))?;

    // Read all tensor data
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| Error::ModelLoad(format!("Failed to read tensor data: {e}")))?;

    // Parse tensors from header
    let header_obj = header
        .as_object()
        .ok_or_else(|| Error::ModelLoad("Header is not a JSON object".to_string()))?;

    let mut tensors = HashMap::new();
    let mut metadata = None;

    for (name, info) in header_obj {
        // Skip metadata entry
        if name == "__metadata__" {
            metadata = Some(info.clone());
            continue;
        }

        let info_obj = info
            .as_object()
            .ok_or_else(|| Error::ModelLoad(format!("Tensor {name} info is not an object")))?;

        // Get dtype
        let dtype_str = info_obj
            .get("dtype")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ModelLoad(format!("Missing dtype for {name}")))?;

        let dtype = match dtype_str {
            "F32" => DataType::F32,
            "F16" => DataType::F16,
            "BF16" => DataType::BF16,
            "I32" => DataType::I32,
            "I64" => DataType::I64,
            _ => return Err(Error::ModelLoad(format!("Unknown dtype: {dtype_str}"))),
        };

        // Get shape
        let shape: Vec<usize> = info_obj
            .get("shape")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::ModelLoad(format!("Missing shape for {name}")))?
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as usize))
            .collect();

        // Get data offsets
        let data_offsets = info_obj
            .get("data_offsets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::ModelLoad(format!("Missing data_offsets for {name}")))?;

        let start = data_offsets
            .first()
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::ModelLoad(format!("Invalid start offset for {name}")))?
            as usize;

        let end = data_offsets
            .get(1)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::ModelLoad(format!("Invalid end offset for {name}")))?
            as usize;

        // Extract tensor data
        if end > data.len() {
            return Err(Error::ModelLoad(format!(
                "Tensor {name} offset {end} exceeds data size {}",
                data.len()
            )));
        }

        let tensor_data = data[start..end].to_vec();

        tensors.insert(
            name.clone(),
            WeightTensor {
                name: name.clone(),
                shape,
                dtype,
                data: tensor_data,
            },
        );
    }

    tracing::info!("Loaded {} tensors from safetensors", tensors.len());

    Ok(ModelWeights { tensors, metadata })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent() {
        let result = load("/nonexistent/path.safetensors");
        assert!(result.is_err());
    }
}
