// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real GPU Backend Implementations
//!
//! No mocks - only production-ready implementations
//!
//! ## Architecture
//! - **WebGPU** (default): Pure Rust, vendor-agnostic, universal
//! - **Vulkan** (optional): Modern compute API
//!
//! CUDA and OpenCL-class dispatch are handled by **barraCuda** / **coralReef** via capability-based IPC.
//! The `cuda_impl` and `opencl_impl` modules are deprecated stubs retained for backward compatibility.
//!
//! ## Capability Layering
//! 1. Base layer: WebGPU (universal, always available, pure Rust)
//! 2. Optimization layer: Vulkan (optional, discovered at runtime)
//! 3. Vendor / legacy: barraCuda/coralReef (external primals via IPC)

/// DEPRECATED S198: `ocl` removed. OpenCL via barraCuda/coralReef IPC.
pub mod opencl_impl;

#[expect(deprecated)]
pub use opencl_impl::{OpenClBackend, OpenClComputeResource};

/// DEPRECATED S197: `cudarc` removed. CUDA via barraCuda/coralReef IPC.
pub mod cuda_impl;

#[expect(deprecated)]
pub use cuda_impl::{CudaBackend, CudaComputeResource};

#[cfg(feature = "vulkan")]
pub mod vulkan_impl;

#[cfg(feature = "vulkan")]
pub use vulkan_impl::{VulkanBackend, VulkanComputeResource};
