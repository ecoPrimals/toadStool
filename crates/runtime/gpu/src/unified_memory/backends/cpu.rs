// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU fallback backend — Always available.
//!
//! Delegates aligned allocation to [`toadstool_hw_safe::AlignedAlloc`],
//! eliminating duplicate unsafe alloc/dealloc code.
//!
//! **Why not `Vec<u8>`?** `Vec` uses default allocator alignment (typically 8-16 bytes).
//! Unified memory backends require 64-byte (cache-line) alignment for DMA and coherent
//! access. `AlignedAlloc` wraps `std::alloc::alloc_zeroed` with a custom layout.

use crate::unified_memory::{
    backend::{BackendAllocation, BackendInitializer, CpuAllocation, UnifiedMemoryBackend},
    types::{BackendType, MemoryFlags, UnifiedMemoryCapabilities},
};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_hw_safe::AlignedAlloc;

/// CPU shared memory backend
///
/// This backend uses standard heap allocation via Rust's allocator.
/// It provides a fallback when no GPU unified memory is available.
///
/// # Characteristics
///
/// - **Always available**: Works on any system
/// - **Coherent**: No synchronization needed
/// - **Fast CPU access**: Direct memory access
/// - **No GPU acceleration**: CPU-only, no actual GPU access
pub struct CpuBackend {
    capabilities: UnifiedMemoryCapabilities,
}

impl CpuBackend {
    /// Create new CPU backend
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`; reserved for future capability probe failures.
    pub const fn new() -> ToadStoolResult<Self> {
        Ok(Self {
            capabilities: UnifiedMemoryCapabilities {
                backend_type: BackendType::Cpu,
                max_allocation_size: Self::MAX_ALLOC,
                zero_copy: true,
                coherent: true,
                cpu_fast_access: true,
                gpu_fast_access: false,
                alignment_requirement: 64,
            },
        })
    }

    #[cfg(target_pointer_width = "64")]
    const MAX_ALLOC: usize = 4 * 1024 * 1024 * 1024; // 4 GB

    #[cfg(not(target_pointer_width = "64"))]
    const MAX_ALLOC: usize = 512 * 1024 * 1024; // 512 MB on 32-bit
}

impl BackendInitializer for CpuBackend {
    async fn try_init() -> ToadStoolResult<Self> {
        Self::new()
    }

    fn is_available() -> bool {
        true
    }
}

impl UnifiedMemoryBackend for CpuBackend {
    fn name(&self) -> &'static str {
        "CPU"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Cpu
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        &self.capabilities
    }

    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        let alloc = AlignedAlloc::new(size, self.capabilities.alignment_requirement)
            .map_err(|e| ToadStoolError::runtime(format!("CPU alloc: {e}")))?;

        tracing::debug!(
            "CPU backend allocated {} bytes (alignment {}, zeroed)",
            alloc.size(),
            self.capabilities.alignment_requirement
        );

        Ok(BackendAllocation::Cpu(CpuAllocation { alloc }))
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match allocation {
            BackendAllocation::Cpu(_alloc) => {
                // AlignedAlloc handles dealloc in Drop — just let it drop
                Ok(())
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for CPU backend",
            )),
        }
    }

    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        match allocation {
            BackendAllocation::Cpu(alloc) => Ok(alloc.ptr()),
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for CPU backend",
            )),
        }
    }

    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8 {
        match allocation {
            BackendAllocation::Cpu(alloc) => alloc.ptr() as *const u8,
            _ => std::ptr::null(),
        }
    }

    async fn sync_cpu_to_device(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn sync_device_to_cpu(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        Ok(())
    }

    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        matches!(allocation, BackendAllocation::Cpu(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cpu_backend_initialization() {
        let backend = CpuBackend::try_init().await;
        assert!(backend.is_ok());

        let backend = backend.unwrap();
        assert_eq!(backend.name(), "CPU");
        assert_eq!(backend.backend_type(), BackendType::Cpu);
        assert!(backend.capabilities().coherent);
    }

    #[tokio::test]
    async fn test_cpu_backend_allocation() {
        let backend = CpuBackend::new().unwrap();

        let mut allocation = backend
            .allocate_unified(4096, MemoryFlags::default())
            .await
            .unwrap();

        assert!(matches!(allocation, BackendAllocation::Cpu(_)));

        let cpu_ptr = backend.map_cpu_ptr(&allocation).await.unwrap();
        let device_ptr = backend.get_device_ptr(&allocation);

        assert!(!cpu_ptr.is_null());
        assert!(!device_ptr.is_null());
        assert_eq!(cpu_ptr as *const u8, device_ptr);

        let BackendAllocation::Cpu(ref mut alloc) = allocation else {
            unreachable!("expected CPU allocation");
        };
        alloc.as_mut_slice().fill(42);
        assert_eq!(alloc.as_mut_slice()[0], 42);

        backend.free_unified(allocation).await.unwrap();
    }

    #[tokio::test]
    async fn test_cpu_backend_sync() {
        let backend = CpuBackend::new().unwrap();
        let allocation = backend
            .allocate_unified(1024, MemoryFlags::default())
            .await
            .unwrap();

        let result = backend.sync_cpu_to_device(&allocation).await;
        assert!(result.is_ok());

        let result = backend.sync_device_to_cpu(&allocation).await;
        assert!(result.is_ok());

        backend.free_unified(allocation).await.unwrap();
    }

    #[test]
    fn test_cpu_backend_always_available() {
        assert!(CpuBackend::is_available());
    }
}
