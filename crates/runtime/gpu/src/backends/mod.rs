// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real GPU Backend Implementations
//!
//! No mocks - only production-ready implementations
//!
//! ## Architecture
//! - **WebGPU** (default): Pure Rust, vendor-agnostic, universal
//! - **Vulkan** (optional): Modern compute API
//!
//! CUDA and OpenCL-class dispatch are handled by `gpu.dispatch.cuda` capability
//! providers via IPC. The `cuda_impl` stub module was removed in S241 — callers
//! should use `discover_capability("gpu.dispatch.cuda")` for CUDA workloads.
//!
//! ## Capability Layering
//! 1. Base layer: WebGPU (universal, always available, pure Rust)
//! 2. Optimization layer: Vulkan (optional, discovered at runtime)
//! 3. Vendor / legacy: `gpu.dispatch.cuda` capability providers (external primals via IPC)

#[cfg(feature = "vulkan")]
pub mod vulkan_impl;

#[cfg(feature = "vulkan")]
pub use vulkan_impl::{VulkanBackend, VulkanComputeResource};
