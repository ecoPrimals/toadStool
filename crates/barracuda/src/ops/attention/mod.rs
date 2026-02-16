//! Scaled Dot-Product Attention - GPU-accelerated implementation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL multi-pass implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! ```text
//! Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) * V
//! ```
//!
//! **Implementation**: 3-pass GPU execution
//! 1. Pass 1: Compute QK^T scores (matrix multiplication)
//! 2. Pass 2: Apply softmax to scores (row-wise)
//! 3. Pass 3: Apply weights to values (weighted sum)
//!
//! **Reference**: "Attention is All You Need" (Vaswani et al., 2017)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let query = Tensor::randn(vec![2, 8, 128, 64]).await?;  // [batch, heads, seq, dim]
//! let key = Tensor::randn(vec![2, 8, 128, 64]).await?;
//! let value = Tensor::randn(vec![2, 8, 128, 64]).await?;
//!
//! let output = query.attention(&key, &value)?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

mod compute;

#[cfg(test)]
mod tests;

/// Attention parameters for WGSL shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AttentionParams {
    pub batch_size: u32,
    pub num_heads: u32,
    pub seq_len: u32,
    pub head_dim: u32,
}

/// Scaled dot-product attention operation
///
/// **Multi-pass GPU implementation**:
/// - Pass 1: QK^T (attention scores)
/// - Pass 2: Softmax (attention weights)
/// - Pass 3: Apply to V (output)
pub struct Attention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
}

impl Attention {
    /// Create new attention operation
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Result<Self> {
        // Validate shapes: all must be [batch, heads, seq_len, head_dim]
        let q_ndim = query.shape().len();
        let k_ndim = key.shape().len();
        let v_ndim = value.shape().len();

        if q_ndim != 4 {
            return Err(BarracudaError::invalid_op(
                "attention",
                format!(
                    "query requires 4D tensor [batch, heads, seq_len, head_dim], got {}D",
                    q_ndim
                ),
            ));
        }
        if k_ndim != 4 {
            return Err(BarracudaError::invalid_op(
                "attention",
                format!(
                    "key requires 4D tensor [batch, heads, seq_len, head_dim], got {}D",
                    k_ndim
                ),
            ));
        }
        if v_ndim != 4 {
            return Err(BarracudaError::invalid_op(
                "attention",
                format!(
                    "value requires 4D tensor [batch, heads, seq_len, head_dim], got {}D",
                    v_ndim
                ),
            ));
        }

        if query.shape() != key.shape() || query.shape() != value.shape() {
            return Err(BarracudaError::shape_mismatch(
                query.shape().to_vec(),
                key.shape().to_vec(),
            ));
        }

        Ok(Self { query, key, value })
    }

    /// Pass 1 shader: Compute QK^T scores
    pub(crate) fn shader_matmul() -> &'static str {
        include_str!("../../shaders/math/attention_matmul.wgsl")
    }

    /// Pass 2 shader: Apply softmax
    pub(crate) fn shader_softmax() -> &'static str {
        include_str!("../../shaders/activation/attention_softmax.wgsl")
    }

    /// Pass 3 shader: Apply weights to values
    pub(crate) fn shader_apply() -> &'static str {
        include_str!("../../shaders/attention/attention_apply.wgsl")
    }

    /// Get query tensor
    pub(crate) fn query(&self) -> &Tensor {
        &self.query
    }

    /// Get key tensor
    pub(crate) fn key(&self) -> &Tensor {
        &self.key
    }

    /// Get value tensor
    pub(crate) fn value(&self) -> &Tensor {
        &self.value
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Scaled dot-product attention
    ///
    /// # Arguments
    ///
    /// * `key` - Key tensor [batch, heads, seq_len, head_dim]
    /// * `value` - Value tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Returns
    ///
    /// Output tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = query.attention(&key, &value)?;
    /// ```
    pub fn attention(self, key: &Self, value: &Self) -> Result<Self> {
        Attention::new(self, key.clone(), value.clone())?.execute()
    }
}
