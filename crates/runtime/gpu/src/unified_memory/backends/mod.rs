// SPDX-License-Identifier: AGPL-3.0-or-later
//! Backend implementations for unified memory

// CPU backend (always available)
pub mod cpu;

// GPU backends (feature-gated)
#[cfg(feature = "vulkan")]
pub mod vulkan;

pub mod opencl;

#[cfg(feature = "webgpu")]
pub mod webgpu;

// Re-exports
pub use cpu::CpuBackend;

#[cfg(feature = "vulkan")]
pub use vulkan::VulkanBackend;

#[expect(deprecated)]
pub use opencl::OpenClBackend;

#[cfg(feature = "webgpu")]
pub use webgpu::WebGpuBackend;
