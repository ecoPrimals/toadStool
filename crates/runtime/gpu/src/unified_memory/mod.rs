// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Unified Memory - Vendor-Agnostic Zero-Copy GPU Compute
//!
//! **Philosophy**: "If it has memory, we can share it" 🍄
//!
//! This module provides vendor-agnostic unified memory that enables zero-copy
//! compute between CPU and GPU across Intel, AMD, and NVIDIA hardware.
//!
//! # Architecture
//!
//! ```text
//! Application Code
//!     ↓
//! UniversalUnifiedMemory (high-level API)
//!     ↓
//! Backend Trait (abstraction)
//!     ↓
//! ┌──────────┬─────────┬─────────┬──────┐
//! │ Vulkan   │ OpenCL  │ WebGPU  │ CPU  │
//! └──────────┴─────────┴─────────┴──────┘
//!     ↓          ↓         ↓        ↓
//! Intel/AMD/NVIDIA (hardware)
//! ```
//!
//! # Core Concepts
//!
//! ## Unified Memory
//!
//! CPU and GPU share the same physical memory - no copies needed:
//!
//! ```no_run
//! use toadstool_runtime_gpu::unified_memory::*;
//!
//! # async fn example() -> toadstool::error::ToadStoolResult<()> {
//! // Initialize unified memory (auto-selects best backend)
//! let memory = UniversalUnifiedMemory::new().await?;
//!
//! // Allocate shared buffer
//! let mut buffer = memory.allocate(4096).await?;
//!
//! // Write from CPU
//! let data = vec![42u8; 1024];
//! buffer.write_async(0, &data).await?;
//!
//! // GPU reads same memory - no copy!
//! let device_ptr = buffer.device_ptr();
//!
//! // Read back from CPU - no copy!
//! let result = buffer.read_async(0, 1024).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Backend Selection
//!
//! Priority order (sovereignty-first):
//! 1. **WebGPU** - Pure Rust, vendor-agnostic (future primary)
//! 2. **Vulkan** - Cross-vendor, modern (current best)
//! 3. **OpenCL** - Cross-vendor, legacy (compatibility)
//! 4. **CPU** - Always available (fallback)
//!
//! ## Zero-Copy Performance
//!
//! Traditional approach (with copies):
//! ```text
//! CPU → Copy 1s → GPU → Compute 0.1s → Copy 1s → CPU
//! Total: 2.1s (95% wasted!)
//! ```
//!
//! Unified memory (zero-copy):
//! ```text
//! Shared Memory → Compute 0.1s
//! Total: 0.1s (21x faster!)
//! ```
//!
//! # Safety
//!
//! All unsafe code is:
//! - Documented with SAFETY comments
//! - Isolated to FFI boundaries
//! - Validated with bounds checking
//! - Protected with proper synchronization
//!
//! # Features
//!
//! - **Async-native**: All operations are `async`, fully concurrent
//! - **Thread-safe**: Uses `Arc`, `RwLock`, `DashMap` for concurrency
//! - **Zero unwraps**: Comprehensive error handling
//! - **Vendor-agnostic**: Works on Intel, AMD, NVIDIA via open standards
//! - **Sovereignty-first**: Prioritizes pure Rust WebGPU backend

// Module declarations
pub mod backend;
pub mod buffer;
pub mod manager;
pub mod types;

// Backend implementations
pub mod backends;

// Re-exports for convenience
pub use backend::UnifiedMemoryBackend;
pub use buffer::UnifiedBuffer;
pub use manager::UniversalUnifiedMemory;
pub use types::*;
