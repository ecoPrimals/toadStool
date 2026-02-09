//! Standard deviation reduction - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

mod compute;

#[cfg(test)]
mod tests;

use crate::error::Result;
use crate::tensor::Tensor;

/// Standard deviation reduction operation
pub struct Std {
    input: Tensor,
    dim: Option<usize>, // None = global std, Some(d) = std along dimension d
    keepdim: bool,      // Whether to keep dimension with size 1
}

impl Std {
    /// Create a new std operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self {
            input,
            dim,
            keepdim,
        }
    }

    /// Get the WGSL shader source for global reduction
    pub(crate) fn wgsl_shader_reduce() -> &'static str {
        include_str!("../../shaders/std_reduce.wgsl")
    }

    /// Get the WGSL shader source for dimension-wise reduction
    pub(crate) fn wgsl_shader_dim() -> &'static str {
        include_str!("../../shaders/std_dim.wgsl")
    }

    /// Execute the std operation
    pub fn execute(self) -> Result<Tensor> {
        compute::execute(self)
    }
}

impl Tensor {
    /// Compute standard deviation (global reduction)
    pub fn std(&self) -> Result<Self> {
        Std::new(self.clone(), None, false).execute()
    }

    /// Compute standard deviation along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to compute std along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn std_dim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Std::new(self.clone(), Some(dim), keepdim).execute()
    }
}
