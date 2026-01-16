//! Normalization Operations
//!
//! Neural network normalization techniques for stable training and inference.
//! All operations run on GPU with multi-pass pipelines for numerical stability.
//!
//! ## Normalization Types (10 variants)
//!
//! ### Softmax (1)
//! - **Softmax** - Converts logits to probabilities (classification)
//!
//! ### Layer Normalization (5 variants)
//! - **LayerNorm** - Standard implementation
//! - **LayerNorm Optimized** - Performance-tuned variant
//! - **LayerNorm Fused** - Single-pass fused operations
//! - **LayerNorm 2-Dispatch** - Two-pass optimized
//! - **LayerNorm Fused V2** - Advanced fused implementation
//!
//! ### Batch/Group/Instance (3)
//! - **BatchNorm** - Normalizes across batch dimension
//! - **GroupNorm** - Group-wise normalization
//! - **InstanceNorm** - Per-instance normalization
//!
//! ### RMS Normalization (1)
//! - **RMSNorm** - Root Mean Square normalization (Transformers)
//!
//! ## Implementation Details
//!
//! All normalization operations use multi-pass GPU pipelines:
//! 1. **Statistics computation** (mean, variance, max)
//! 2. **Normalization** (subtract mean, divide by std)
//! 3. **Affine transform** (scale + shift, optional)
//!
//! ## Deep Debt Compliance
//!
//! - ✅ **Runtime Configuration**: All sizes/shapes configurable
//! - ✅ **No Hardcoding**: Parameters passed at runtime
//! - ✅ **Numerical Stability**: Multi-pass for precision
//! - ✅ **Pure Rust**: Zero unsafe code

// Module structure
mod softmax;
mod layernorm;
mod batchnorm;
mod groupnorm;
mod instance_norm;
mod rms_norm;

// No re-exports needed - methods are impl blocks on WgpuExecutor
