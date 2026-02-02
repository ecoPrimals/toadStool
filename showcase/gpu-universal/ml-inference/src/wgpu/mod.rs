//! WGPU GPU Executor - Modern Idiomatic Rust Implementation
//!
//! This module provides a pure Rust GPU compute abstraction using WebGPU (wgpu).
//!
//! **Design Philosophy**:
//! - **Zero FFI**: No C/C++ foreign function calls
//! - **Zero Unsafe** (in our code): Safe Rust throughout
//! - **Modern Async/Await**: Idiomatic asynchronous patterns
//! - **Deep Debt Compliance**: Runtime discovery, no hardcoding
//! - **Modular Architecture**: Logical separation of concerns
//!
//! **Module Structure**:
//! ```text
//! wgpu/
//!   ├── mod.rs           - This file, public API
//!   ├── types.rs         - Configuration types and enums
//!   ├── executor.rs      - Main GPU coordinator
//!   ├── utils.rs         - Common helpers (eliminates boilerplate)
//!   ├── activations.rs   - Activation functions (ReLU, Sigmoid, Tanh, etc.)
//!   ├── basic_ops.rs     - Basic operations (MatMul, Add, etc.)
//!   ├── normalization.rs - Normalization layers (LayerNorm, BatchNorm, etc.)
//!   ├── pooling.rs       - Pooling operations
//!   ├── advanced_ops.rs  - Advanced operations (Gather, Scatter, etc.)
//!   └── training.rs      - Training operations (Optimizers, Loss functions)
//! ```
//!
//! **Before Refactor**: 5,116 lines in one file
//! **After Refactor**: ~500 lines per module (maintainable!)
//!
//! **Key Improvements**:
//! 1. **Eliminated Boilerplate**: Extracted common patterns into `utils.rs`
//! 2. **Type Safety**: Moved configurations to `types.rs`
//! 3. **Logical Grouping**: Operations grouped by function
//! 4. **Modern Patterns**: Async/await, Result, idiomatic error handling
//! 5. **Deep Debt**: Runtime discovery, no hardcoded GPU requirements

// Re-export types for public API
pub use types::*;

// Re-export executor
pub use executor::{GpuCapabilities, WgpuExecutor};

// Internal modules
pub(crate) mod activations;
pub(crate) mod advanced_ops;
pub(crate) mod async_executor;
pub(crate) mod basic_ops;
pub(crate) mod data_ops;
mod executor;
pub(crate) mod matmul_strategy;
pub(crate) mod normalization;
pub(crate) mod pooling;
pub(crate) mod reductions;
pub(crate) mod regularization;
pub(crate) mod tensor_ops; // NEW: Week 3 neuromorphic operations
pub(crate) mod training;
mod types;
pub(crate) mod utils;

// Re-export async execution framework
pub use async_executor::{AsyncBatch, AsyncOp, AsyncPipeline, AsyncStats, GpuVendor};

// Re-export MatMul strategy selection
pub use matmul_strategy::MatMulStrategy;

// Re-export commonly used items for convenience
pub use anyhow::{Context, Result};
pub use wgpu::util::DeviceExt;

// Re-export neuromorphic tensor operations (Week 3 + Phase 2 + Phase 3)
pub use tensor_ops::{
    // Phase 2 - 10 ops
    Abs,
    // Phase 1 (Week 3) - 7 ops
    Argmax,
    Cast,
    Clamp,
    // Phase 3 - 15 ops
    Cumsum,
    Exp,
    Expand,
    LayerNorm,
    LogSoftmax,
    Max,
    Mean,
    Min,
    Norm,
    Pad,
    PadMode,
    Pow,
    Prod,
    ReLU,
    Reshape,
    Sigmoid,
    Slice,
    Softmax,
    Sqrt,
    Squeeze,
    Std,
    Sum,
    TopK,
    Transpose,
    Unsqueeze,
    Var,
    Where,
    GELU,
};
