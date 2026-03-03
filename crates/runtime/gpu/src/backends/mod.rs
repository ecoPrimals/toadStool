// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real GPU Backend Implementations
//!
//! No mocks - only production-ready implementations
//!
//! ## Architecture
//! - **WebGPU** (default): Pure Rust, vendor-agnostic, universal
//! - **OpenCL** (optional): Universal, works on NVIDIA/AMD/Intel
//! - **CUDA** (optional): High performance for NVIDIA (Python AI compatibility)
//! - **Vulkan** (optional): Modern compute API
//!
//! ## DEEP DEBT EVOLUTION NOTES:
//!
//! **Why Feature Gates Here Are CORRECT:**
//! - Backend implementations depend on external C libraries (OpenCL, CUDA)
//! - Feature gates allow **optional optimization** without breaking universal support
//! - Default WebGPU backend (wgpu) works everywhere, **no feature required**
//! - Backends are **runtime discovered** when features enabled, gracefully absent when not
//!
//! This is NOT hardcoding - it's **capability layering**:
//! 1. Base layer: WebGPU (universal, always available)
//! 2. Optimization layers: CUDA/OpenCL/Vulkan (optional, discovered at runtime)
//!
//! ## Evolution Strategy
//! - 2025: WebGPU default + optional CUDA for Python AI
//! - 2026+: WebGPU as AI libraries mature
//! - 2027+: Pure WebGPU (may drop CUDA dependency)

#[cfg(feature = "opencl")]
pub mod opencl_impl;

#[cfg(feature = "opencl")]
pub use opencl_impl::{OpenClBackend, OpenClComputeResource};

#[cfg(feature = "cuda")]
pub mod cuda_impl;

#[cfg(feature = "cuda")]
pub use cuda_impl::{CudaBackend, CudaComputeResource};

#[cfg(feature = "vulkan")]
pub mod vulkan_impl;

#[cfg(feature = "vulkan")]
pub use vulkan_impl::{VulkanBackend, VulkanComputeResource};
