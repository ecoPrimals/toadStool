//! Masked Select - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! NOTE: This implementation requires prefix sum computation.
//! For now, we compute prefix sum on CPU. A full GPU implementation
//! would require a parallel scan operation.

mod compute;

#[cfg(test)]
mod tests;

use crate::error::Result;
use crate::tensor::Tensor;

/// Masked select operation
pub struct MaskedSelect {
    input: Tensor,
    mask: Tensor,
}

impl MaskedSelect {
    /// Create a new masked select operation
    pub fn new(input: Tensor, mask: Tensor) -> Result<Self> {
        if input.shape() != mask.shape() {
            return Err(crate::error::BarracudaError::ShapeMismatch {
                expected: input.shape().to_vec(),
                actual: mask.shape().to_vec(),
            });
        }

        Ok(Self {
            input,
            mask,
        })
    }

    /// Get the WGSL shader source
    pub(super) fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/masked_select.wgsl")
    }

    pub(super) fn prefix_sum_shader() -> &'static str {
        include_str!("../../shaders/prefix_sum.wgsl")
    }

    pub(super) fn mask_convert_shader() -> &'static str {
        include_str!("../../shaders/mask_convert.wgsl")
    }

    /// Get input tensor
    pub(super) fn input(&self) -> &Tensor {
        &self.input
    }

    /// Get mask tensor
    pub(super) fn mask(&self) -> &Tensor {
        &self.mask
    }

    /// Execute the masked select operation
    pub fn execute(self) -> Result<Tensor> {
        compute::execute_masked_select(self)
    }
}
