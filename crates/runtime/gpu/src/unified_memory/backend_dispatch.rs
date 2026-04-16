// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch for [`UnifiedMemoryBackend`](super::backend::UnifiedMemoryBackend).

use super::backend::{BackendAllocation, UnifiedMemoryBackend};
use super::backends::CpuBackend;
#[cfg(feature = "vulkan")]
use super::backends::VulkanBackend;
#[cfg(feature = "webgpu")]
use super::backends::WebGpuBackend;
use super::types::{AccessPattern, BackendType, MemoryFlags, UnifiedMemoryCapabilities};
use toadstool::error::ToadStoolResult;

/// Closed set of unified memory backend implementations used by the runtime.
pub enum UnifiedMemoryBackendDispatch {
    /// CPU heap (aligned) fallback.
    Cpu(CpuBackend),
    /// `WebGPU` (`wgpu`) path.
    #[cfg(feature = "webgpu")]
    WebGpu(WebGpuBackend),
    /// Vulkan-oriented backend (often via `wgpu`).
    #[cfg(feature = "vulkan")]
    Vulkan(VulkanBackend),
}

impl UnifiedMemoryBackend for UnifiedMemoryBackendDispatch {
    fn name(&self) -> &'static str {
        match self {
            Self::Cpu(b) => b.name(),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.name(),
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.name(),
        }
    }

    fn backend_type(&self) -> BackendType {
        match self {
            Self::Cpu(b) => b.backend_type(),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.backend_type(),
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.backend_type(),
        }
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        match self {
            Self::Cpu(b) => b.capabilities(),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.capabilities(),
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.capabilities(),
        }
    }

    async fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        match self {
            Self::Cpu(b) => b.allocate_unified(size, flags).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.allocate_unified(size, flags).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.allocate_unified(size, flags).await,
        }
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match self {
            Self::Cpu(b) => b.free_unified(allocation).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.free_unified(allocation).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.free_unified(allocation).await,
        }
    }

    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        match self {
            Self::Cpu(b) => b.map_cpu_ptr(allocation).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.map_cpu_ptr(allocation).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.map_cpu_ptr(allocation).await,
        }
    }

    async fn unmap_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        match self {
            Self::Cpu(b) => b.unmap_cpu_ptr(allocation).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.unmap_cpu_ptr(allocation).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.unmap_cpu_ptr(allocation).await,
        }
    }

    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8 {
        match self {
            Self::Cpu(b) => b.get_device_ptr(allocation),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.get_device_ptr(allocation),
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.get_device_ptr(allocation),
        }
    }

    async fn sync_cpu_to_device(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        match self {
            Self::Cpu(b) => b.sync_cpu_to_device(allocation).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.sync_cpu_to_device(allocation).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.sync_cpu_to_device(allocation).await,
        }
    }

    async fn sync_device_to_cpu(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        match self {
            Self::Cpu(b) => b.sync_device_to_cpu(allocation).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.sync_device_to_cpu(allocation).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.sync_device_to_cpu(allocation).await,
        }
    }

    async fn optimize_for_pattern(
        &self,
        allocation: &BackendAllocation,
        pattern: AccessPattern,
    ) -> ToadStoolResult<()> {
        match self {
            Self::Cpu(b) => b.optimize_for_pattern(allocation, pattern).await,
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.optimize_for_pattern(allocation, pattern).await,
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.optimize_for_pattern(allocation, pattern).await,
        }
    }

    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        match self {
            Self::Cpu(b) => b.is_valid(allocation),
            #[cfg(feature = "webgpu")]
            Self::WebGpu(b) => b.is_valid(allocation),
            #[cfg(feature = "vulkan")]
            Self::Vulkan(b) => b.is_valid(allocation),
        }
    }
}
