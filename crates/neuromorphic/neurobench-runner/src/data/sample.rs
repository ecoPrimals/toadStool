// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single inference sample

/// A sample for inference
#[derive(Debug, Clone)]
pub struct Sample {
    /// Input data (raw bytes, to be interpreted by model)
    pub input: Vec<u8>,
    /// Ground truth label (class index for classification, 0 for regression)
    pub label: usize,
    /// Optional sample identifier
    pub id: Option<String>,
}

impl Sample {
    /// Create sample with f32 input (will be converted to bytes)
    #[must_use]
    pub fn from_f32(data: &[f32], label: usize, id: Option<String>) -> Self {
        let input: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        Self { input, label, id }
    }

    /// Get input as f32 slice
    #[must_use]
    pub fn as_f32(&self) -> Vec<f32> {
        self.input
            .chunks_exact(4)
            .map(|chunk| {
                let bytes: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(bytes)
            })
            .collect()
    }
}
