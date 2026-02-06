//! Variance reduction - Pure WGSL
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

/// Variance reduction operation
pub struct Variance {
    input: Tensor,
    dim: Option<usize>,  // None = global variance, Some(d) = variance along dimension d
    keepdim: bool,       // Whether to keep dimension with size 1
}

impl Variance {
    /// Create a new variance operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self { input, dim, keepdim }
    }

    /// Get the WGSL shader source for global reduction
    pub(super) fn wgsl_shader_reduce() -> &'static str {
        include_str!("../../shaders/variance_reduce.wgsl")
    }

    /// Get the WGSL shader source for dimension-wise reduction
    pub(super) fn wgsl_shader_dim() -> &'static str {
        include_str!("../../shaders/variance_dim.wgsl")
    }

    /// Get input tensor
    pub(super) fn input(&self) -> &Tensor {
        &self.input
    }

    /// Get dimension
    pub(super) fn dim(&self) -> Option<usize> {
        self.dim
    }

    /// Get keepdim flag
    pub(super) fn keepdim(&self) -> bool {
        self.keepdim
    }
}

impl Tensor {
    /// Compute variance (global reduction)
    pub fn variance(&self) -> Result<Self> {
        Variance::new(self.clone(), None, false).execute()
    }

    /// Compute variance along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to compute variance along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn variance_dim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Variance::new(self.clone(), Some(dim), keepdim).execute()
    }

    /// Compute variance (legacy method for backward compatibility)
    pub fn var(self) -> Result<Self> {
        Variance::new(self, None, false).execute()
    }
}
