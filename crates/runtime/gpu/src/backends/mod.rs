//! Real GPU Backend Implementations
//!
//! No mocks - only production-ready implementations
//!
//! ## Architecture
//! - **OpenCL**: Universal, works on NVIDIA/AMD/Intel
//! - **CUDA**: High performance for NVIDIA (Python AI compatibility)
//! - **WebGPU**: Pure Rust, vendor-agnostic (future primary)
//!
//! ## Evolution Strategy
//! - 2025: CUDA for Python AI (PyTorch, TensorFlow)
//! - 2026+: WebGPU as AI libraries mature
//! - 2027+: Pure WebGPU (drop CUDA dependency)

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
