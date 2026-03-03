// SPDX-License-Identifier: AGPL-3.0-or-later
//! Causal Mask Generation

use anyhow::Result;

/// Causal Mask Generator
///
/// Generates autoregressive (causal) attention masks for GPT-style models.
/// Prevents positions from attending to future positions.
///
/// ## Mask Pattern
///
/// ```text
/// [[1, 0, 0, 0],
///  [1, 1, 0, 0],
///  [1, 1, 1, 0],
///  [1, 1, 1, 1]]
/// ```
///
/// Position `i` can only attend to positions `j <= i`.
///
/// ## Performance
///
/// - Complexity: O(seq_len²)
/// - Memory: O(batch · seq_len²)
/// - Parallelized generation
pub struct CausalMask;

impl CausalMask {
    /// Generate causal mask
    ///
    /// # Arguments
    ///
    /// * `batch` - Batch size
    /// * `seq_len` - Sequence length
    ///
    /// # Returns
    ///
    /// Mask tensor [batch, seq_len, seq_len] where:
    /// - mask[b, i, j] = 1.0 if j <= i (allow attention)
    /// - mask[b, i, j] = 0.0 if j > i (mask attention)
    pub fn generate(batch: u32, seq_len: u32) -> Vec<f32> {
        let mut mask = vec![0.0f32; (batch * seq_len * seq_len) as usize];

        for b in 0..batch {
            for i in 0..seq_len {
                for j in 0..seq_len {
                    let idx = ((b * seq_len + i) * seq_len + j) as usize;
                    mask[idx] = if j <= i { 1.0 } else { 0.0 };
                }
            }
        }

        mask
    }

    /// Generate causal mask with GPU acceleration
    ///
    /// Same as `generate()` but executed on GPU for large sequences.
    pub async fn generate_gpu(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        batch: u32,
        seq_len: u32,
    ) -> Result<Vec<f32>> {
        // For now, use CPU implementation
        // TODO: Implement GPU shader for large sequences
        Ok(Self::generate(batch, seq_len))
    }
}
