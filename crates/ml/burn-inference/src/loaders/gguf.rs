// SPDX-License-Identifier: AGPL-3.0-only
//! GGUF model file loader
//!
//! Loads model weights from llama.cpp GGUF format.
//! GGUF supports quantized weights (`Q4_0`, `Q4_1`, `Q5_0`, `Q5_1`, `Q8_0`, etc.)
//!
//! ## Format Overview
//!
//! ```text
//! GGUF File Structure:
//! ├── Magic (4 bytes): "GGUF"
//! ├── Version (4 bytes): u32
//! ├── Tensor count (8 bytes): u64
//! ├── Metadata KV count (8 bytes): u64
//! ├── Metadata KV pairs
//! ├── Tensor infos
//! └── Tensor data (aligned)
//! ```
//!
//! ## Supported Quantization Types
//!
//! - F32, F16 - Full precision
//! - `Q4_0`, `Q4_1` - 4-bit quantization
//! - `Q5_0`, `Q5_1` - 5-bit quantization
//! - `Q8_0` - 8-bit quantization

use super::{DataType, ModelWeights, WeightTensor};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// GGUF magic bytes
const GGUF_MAGIC: [u8; 4] = [b'G', b'G', b'U', b'F'];

/// GGUF supported versions
const GGUF_VERSION_2: u32 = 2;
const GGUF_VERSION_3: u32 = 3;

/// GGUF data types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum GgufType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
    Q8_K = 15,
    I8 = 16,
    I16 = 17,
    I32 = 18,
}

impl GgufType {
    const fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::F32),
            1 => Some(Self::F16),
            2 => Some(Self::Q4_0),
            3 => Some(Self::Q4_1),
            6 => Some(Self::Q5_0),
            7 => Some(Self::Q5_1),
            8 => Some(Self::Q8_0),
            9 => Some(Self::Q8_1),
            10 => Some(Self::Q2_K),
            11 => Some(Self::Q3_K),
            12 => Some(Self::Q4_K),
            13 => Some(Self::Q5_K),
            14 => Some(Self::Q6_K),
            15 => Some(Self::Q8_K),
            16 => Some(Self::I8),
            17 => Some(Self::I16),
            18 => Some(Self::I32),
            _ => None,
        }
    }

    /// Get bytes per element (for non-quantized types)
    const fn element_size(&self) -> usize {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::F16 | Self::I16 => 2,
            Self::I8 => 1,
            // Quantized types are per-block, not per-element
            _ => 0,
        }
    }

    /// Get block size for quantized types
    const fn block_size(&self) -> usize {
        match self {
            Self::Q4_0 | Self::Q4_1 | Self::Q5_0 | Self::Q5_1 | Self::Q8_0 | Self::Q8_1 => 32,
            Self::Q2_K | Self::Q3_K | Self::Q4_K | Self::Q5_K | Self::Q6_K | Self::Q8_K => 256,
            _ => 1,
        }
    }

    /// Get bytes per block for quantized types
    const fn bytes_per_block(&self) -> usize {
        match self {
            Self::F32 => 4,
            Self::F16 => 2,
            Self::Q4_0 => 18, // 2 bytes scale + 16 bytes data (32 elements)
            Self::Q4_1 => 20, // 2 bytes scale + 2 bytes min + 16 bytes data
            Self::Q5_0 => 22, // 2 bytes scale + 4 bytes high bits + 16 bytes low
            Self::Q5_1 => 24,
            Self::Q8_0 => 34, // 2 bytes scale + 32 bytes data
            Self::Q8_1 => 36,
            Self::Q2_K => 84,
            Self::Q3_K => 110,
            Self::Q4_K => 144,
            Self::Q5_K => 176,
            Self::Q6_K => 210,
            Self::Q8_K => 292,
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I32 => 4,
        }
    }

    const fn to_data_type(self) -> DataType {
        match self {
            Self::F32 => DataType::F32,
            Self::F16 => DataType::F16,
            _ => DataType::F32, // Quantized types will be dequantized to F32
        }
    }
}

/// GGUF metadata value types
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum GgufMetaType {
    U8 = 0,
    I8 = 1,
    U16 = 2,
    I16 = 3,
    U32 = 4,
    I32 = 5,
    F32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    U64 = 10,
    I64 = 11,
    F64 = 12,
}

/// Tensor info from GGUF header
#[derive(Debug)]
struct GgufTensorInfo {
    name: String,
    #[expect(dead_code, reason = "GGUF format field; dims.len() provides equivalent info")]
    n_dims: u32,
    dims: Vec<u64>,
    gguf_type: GgufType,
    offset: u64,
}

/// Load weights from a GGUF file
pub fn load<P: AsRef<Path>>(path: P) -> Result<ModelWeights> {
    let path = path.as_ref();
    tracing::info!("Loading GGUF from {}", path.display());

    let file = File::open(path).map_err(Error::Io)?;
    let mut reader = BufReader::new(file);

    // Read and verify magic
    let mut magic = [0u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|e| Error::ModelLoad(format!("Failed to read magic: {e}")))?;

    if magic != GGUF_MAGIC {
        return Err(Error::ModelLoad(format!(
            "Invalid GGUF magic: {magic:?} (expected {GGUF_MAGIC:?})"
        )));
    }

    // Read version
    let version = read_u32(&mut reader)?;
    if version != GGUF_VERSION_2 && version != GGUF_VERSION_3 {
        return Err(Error::ModelLoad(format!(
            "Unsupported GGUF version: {version} (supported: 2, 3)"
        )));
    }
    tracing::debug!("GGUF version: {}", version);

    // Read counts
    let tensor_count = read_u64(&mut reader)?;
    let metadata_kv_count = read_u64(&mut reader)?;
    tracing::debug!(
        "GGUF: {} tensors, {} metadata entries",
        tensor_count,
        metadata_kv_count
    );

    // Skip metadata (we don't need it for weights)
    let metadata = skip_metadata(&mut reader, metadata_kv_count)?;

    // Read tensor infos
    let tensor_infos = read_tensor_infos(&mut reader, tensor_count)?;

    // Calculate alignment (GGUF uses 32-byte alignment)
    let alignment: u64 = 32;

    // Get current position and align to data start
    let header_end = reader
        .stream_position()
        .map_err(|e| Error::ModelLoad(format!("Failed to get stream position: {e}")))?;

    let data_offset = header_end.div_ceil(alignment) * alignment;

    // Read tensor data
    let mut tensors = HashMap::new();

    for info in tensor_infos {
        // Seek to tensor data
        reader
            .seek(SeekFrom::Start(data_offset + info.offset))
            .map_err(|e| {
                Error::ModelLoad(format!("Failed to seek to tensor {}: {e}", info.name))
            })?;

        // Calculate data size
        let numel: u64 = info.dims.iter().product();
        let data_size = if info.gguf_type.element_size() > 0 {
            numel as usize * info.gguf_type.element_size()
        } else {
            // Quantized type
            let n_blocks = (numel as usize).div_ceil(info.gguf_type.block_size());
            n_blocks * info.gguf_type.bytes_per_block()
        };

        // Read tensor data
        let mut data = vec![0u8; data_size];
        reader
            .read_exact(&mut data)
            .map_err(|e| Error::ModelLoad(format!("Failed to read tensor {}: {e}", info.name)))?;

        let shape: Vec<usize> = info.dims.iter().map(|&d| d as usize).collect();

        tensors.insert(
            info.name.clone(),
            WeightTensor {
                name: info.name,
                shape,
                dtype: info.gguf_type.to_data_type(),
                data,
            },
        );
    }

    tracing::info!(
        "Loaded {} tensors from GGUF (version {})",
        tensors.len(),
        version
    );

    Ok(ModelWeights {
        tensors,
        metadata: Some(serde_json::to_value(metadata).unwrap_or(serde_json::Value::Null)),
    })
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| Error::ModelLoad(format!("Failed to read u32: {e}")))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader
        .read_exact(&mut buf)
        .map_err(|e| Error::ModelLoad(format!("Failed to read u64: {e}")))?;
    Ok(u64::from_le_bytes(buf))
}

fn read_string<R: Read>(reader: &mut R) -> Result<String> {
    let len = read_u64(reader)? as usize;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .map_err(|e| Error::ModelLoad(format!("Failed to read string: {e}")))?;
    String::from_utf8(buf).map_err(|e| Error::ModelLoad(format!("Invalid UTF-8 string: {e}")))
}

fn skip_metadata<R: Read>(reader: &mut R, count: u64) -> Result<HashMap<String, String>> {
    let mut metadata = HashMap::new();

    for _ in 0..count {
        // Read key
        let key = read_string(reader)?;

        // Read value type
        let value_type = read_u32(reader)?;

        // Read value based on type
        let value_str = match value_type {
            0 => {
                // U8
                let mut buf = [0u8; 1];
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| Error::ModelLoad(format!("Failed to read U8: {e}")))?;
                buf[0].to_string()
            }
            1 => {
                // I8
                let mut buf = [0u8; 1];
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| Error::ModelLoad(format!("Failed to read I8: {e}")))?;
                (buf[0] as i8).to_string()
            }
            2 => {
                // U16
                let mut buf = [0u8; 2];
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| Error::ModelLoad(format!("Failed to read U16: {e}")))?;
                u16::from_le_bytes(buf).to_string()
            }
            3 => {
                // I16
                let mut buf = [0u8; 2];
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| Error::ModelLoad(format!("Failed to read I16: {e}")))?;
                i16::from_le_bytes(buf).to_string()
            }
            4 => {
                // U32
                read_u32(reader)?.to_string()
            }
            5 => {
                // I32
                let v = read_u32(reader)?;
                (v as i32).to_string()
            }
            6 => {
                // F32
                let v = read_u32(reader)?;
                f32::from_bits(v).to_string()
            }
            7 => {
                // Bool
                let mut buf = [0u8; 1];
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| Error::ModelLoad(format!("Failed to read Bool: {e}")))?;
                (buf[0] != 0).to_string()
            }
            8 => {
                // String
                read_string(reader)?
            }
            9 => {
                // Array - skip
                let elem_type = read_u32(reader)?;
                let len = read_u64(reader)?;
                let elem_size = match elem_type {
                    0..=1 | 7 => 1,
                    2..=3 => 2,
                    4..=6 => 4,
                    8 => {
                        // Array of strings - skip each
                        for _ in 0..len {
                            let _ = read_string(reader)?;
                        }
                        0
                    }
                    10..=12 => 8,
                    _ => 0,
                };
                if elem_size > 0 {
                    let mut buf = vec![0u8; len as usize * elem_size];
                    reader
                        .read_exact(&mut buf)
                        .map_err(|e| Error::ModelLoad(format!("Failed to skip array: {e}")))?;
                }
                format!("[array len={len}]")
            }
            10 => {
                // U64
                read_u64(reader)?.to_string()
            }
            11 => {
                // I64
                let v = read_u64(reader)?;
                (v as i64).to_string()
            }
            12 => {
                // F64
                let v = read_u64(reader)?;
                f64::from_bits(v).to_string()
            }
            _ => {
                return Err(Error::ModelLoad(format!(
                    "Unknown metadata type: {value_type}"
                )));
            }
        };

        metadata.insert(key, value_str);
    }

    Ok(metadata)
}

fn read_tensor_infos<R: Read>(reader: &mut R, count: u64) -> Result<Vec<GgufTensorInfo>> {
    let mut infos = Vec::with_capacity(count as usize);

    for _ in 0..count {
        // Read name
        let name = read_string(reader)?;

        // Read number of dimensions
        let n_dims = read_u32(reader)?;

        // Read dimensions
        let mut dims = Vec::with_capacity(n_dims as usize);
        for _ in 0..n_dims {
            dims.push(read_u64(reader)?);
        }

        // Read type
        let type_id = read_u32(reader)?;
        let gguf_type = GgufType::from_u32(type_id).ok_or_else(|| {
            Error::ModelLoad(format!("Unknown GGUF type: {type_id} for tensor {name}"))
        })?;

        // Read offset
        let offset = read_u64(reader)?;

        infos.push(GgufTensorInfo {
            name,
            n_dims,
            dims,
            gguf_type,
            offset,
        });
    }

    Ok(infos)
}

/// Dequantize `Q4_0` data to f32
#[must_use]
pub fn dequantize_q4_0(data: &[u8], numel: usize) -> Vec<f32> {
    let block_size = 32;
    let n_blocks = numel.div_ceil(block_size);
    let mut output = Vec::with_capacity(numel);

    for block_idx in 0..n_blocks {
        let block_offset = block_idx * 18; // 2 bytes scale + 16 bytes data
        if block_offset + 18 > data.len() {
            break;
        }

        // Read scale (f16)
        let scale_bytes = [data[block_offset], data[block_offset + 1]];
        let scale = half::f16::from_le_bytes(scale_bytes).to_f32();

        // Read quantized values (4 bits each)
        for i in 0..16 {
            let byte = data[block_offset + 2 + i];
            let q0 = (byte & 0x0F) as i8 - 8;
            let q1 = ((byte >> 4) & 0x0F) as i8 - 8;

            output.push(scale * f32::from(q0));
            if output.len() < numel {
                output.push(scale * f32::from(q1));
            }
        }
    }

    output.truncate(numel);
    output
}

/// Dequantize `Q8_0` data to f32
#[must_use]
pub fn dequantize_q8_0(data: &[u8], numel: usize) -> Vec<f32> {
    let block_size = 32;
    let n_blocks = numel.div_ceil(block_size);
    let mut output = Vec::with_capacity(numel);

    for block_idx in 0..n_blocks {
        let block_offset = block_idx * 34; // 2 bytes scale + 32 bytes data
        if block_offset + 34 > data.len() {
            break;
        }

        // Read scale (f16)
        let scale_bytes = [data[block_offset], data[block_offset + 1]];
        let scale = half::f16::from_le_bytes(scale_bytes).to_f32();

        // Read quantized values (8 bits each)
        for i in 0..32 {
            if output.len() >= numel {
                break;
            }
            let q = data[block_offset + 2 + i] as i8;
            output.push(scale * f32::from(q));
        }
    }

    output.truncate(numel);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gguf_type_from_u32() {
        assert_eq!(GgufType::from_u32(0), Some(GgufType::F32));
        assert_eq!(GgufType::from_u32(1), Some(GgufType::F16));
        assert_eq!(GgufType::from_u32(2), Some(GgufType::Q4_0));
        assert_eq!(GgufType::from_u32(8), Some(GgufType::Q8_0));
        assert_eq!(GgufType::from_u32(100), None);
    }

    #[test]
    fn test_block_sizes() {
        assert_eq!(GgufType::Q4_0.block_size(), 32);
        assert_eq!(GgufType::Q8_0.block_size(), 32);
        assert_eq!(GgufType::Q4_K.block_size(), 256);
    }

    #[test]
    fn test_load_nonexistent() {
        let result = load("/nonexistent/path.gguf");
        assert!(result.is_err());
    }

    #[test]
    fn test_dequantize_q4_0() {
        // Create a simple Q4_0 block
        // scale = 1.0 (as f16), then 16 bytes of quantized data
        let mut data = vec![0u8; 18];
        data[0] = 0x00; // f16 1.0 = 0x3C00
        data[1] = 0x3C;
        // Set some quantized values (4-bit pairs)
        data[2] = 0x88; // Two 8s (which become 0 after -8 offset)

        let result = dequantize_q4_0(&data, 2);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.0).abs() < 0.1);
        assert!((result[1] - 0.0).abs() < 0.1);
    }
}
