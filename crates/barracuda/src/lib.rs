//! # barraCUDA: Hardware-Agnostic Tensor Compute
//!
//! **Deep Debt Excellence**: Zero duplication, pure capability-based compute
//!
//! ## Philosophy
//!
//! - ✅ **Hardware-Agnostic**: One API, works on any device (GPU/CPU/NPU/TPU)
//! - ✅ **Pure WGSL**: WGSL shaders ONLY (wgpu handles all backends)
//! - ✅ **Automatic Fallback**: wgpu uses CPU when GPU unavailable
//! - ✅ **Zero Duplication**: Single WGSL implementation per operation
//! - ✅ **Runtime Discovery**: wgpu selects best available backend
//! - ✅ **Simple**: No separate CPU code, no trait abstractions
//! - ✅ **Pure Rust**: Zero unsafe in barraCUDA core, zero FFI
//!
//! ## Architecture
//!
//! ```text
//! User Code: Tensor<f32>
//!     ↓
//! Operation (WGSL shader)
//!     ↓
//! WgpuDevice
//!     ↓
//! wgpu Backend Selection (automatic):
//! ├── Vulkan (NVIDIA, AMD, Intel GPU)
//! ├── Metal (Apple GPU)
//! ├── DX12 (Windows GPU)
//! └── Software Rasterizer (CPU fallback)
//!
//! Same WGSL code runs on ALL backends!
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use barracuda::prelude::*;
//!
//! // Auto-discovers best device (GPU if available, CPU fallback)
//! let x = Tensor::randn([128, 256])?;
//! let y = Tensor::randn([256, 512])?;
//!
//! // Operations execute on discovered device (WGSL on GPU, Rayon on CPU)
//! let z = x.matmul(&y)?;
//! let activated = z.relu()?;
//! let normalized = activated.softmax(0)?;
//!
//! println!("Device: {}", x.device().name());
//! // "NVIDIA GeForce RTX 4090" or "AMD Radeon RX 7900" or "CPU (16 cores)"
//! ```
//!
//! ## Deep Debt Elimination
//!
//! **Before** (architectural debt):
//! - Separate CPU and GPU implementations
//! - User must choose backend explicitly
//! - WGSL shaders existed but weren't used by operations
//! - Duplication: Same logic in CPU and WGSL
//!
//! **After** (unified):
//! - Single Tensor API works everywhere
//! - Automatic device discovery and fallback
//! - All WGSL shaders properly utilized
//! - Zero duplication: One implementation per op

#![deny(unsafe_code)] // Zero unsafe in barraCUDA core!

pub mod device;
pub mod error;
pub mod ops;
pub mod tensor;

// Re-export commonly used operations
pub use ops::spike_encode::spike_encode;
pub use ops::spike_decode::spike_decode;
pub use ops::lif_neuron::lif_neuron;
pub use ops::temporal_pool::temporal_pool;
pub use ops::sparse_matmul_quantized::sparse_matmul_quantized;
pub use ops::pattern_match::pattern_match;
pub use ops::gc_content::gc_content;
pub use ops::complexity_filter::complexity_filter;

/// Prelude: Common imports for using barracuda
pub mod prelude {
    pub use crate::device::{Auto, WgpuDevice};
    pub use crate::error::{BarracudaError, Result};
    pub use crate::tensor::Tensor;
}
