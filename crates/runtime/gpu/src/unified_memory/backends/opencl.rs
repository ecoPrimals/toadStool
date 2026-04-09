// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL unified memory backend — **DEPRECATED S198**
//!
//! Direct OpenCL SVM via `ocl` has been removed. Use **barraCuda** / **coralReef**
//! for GPU memory and dispatch via IPC.

use crate::unified_memory::backend::{BackendAllocation, BackendInitializer, UnifiedMemoryBackend};
use crate::unified_memory::types::{BackendType, MemoryFlags, UnifiedMemoryCapabilities};
use async_trait::async_trait;
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Stub backend retained for API compatibility (initialization always fails).
pub struct OpenClBackend {
    _private: (),
}

impl BackendInitializer for OpenClBackend {
    async fn try_init() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "OpenCL unified memory removed (S198): use barraCuda/coralReef via IPC.",
        ))
    }

    fn is_available() -> bool {
        false
    }
}

#[async_trait]
impl UnifiedMemoryBackend for OpenClBackend {
    fn name(&self) -> &'static str {
        "OpenCL (deprecated)"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::OpenCL
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        static CAPS: UnifiedMemoryCapabilities = UnifiedMemoryCapabilities {
            backend_type: BackendType::OpenCL,
            max_allocation_size: 0,
            zero_copy: false,
            coherent: false,
            cpu_fast_access: false,
            gpu_fast_access: false,
            alignment_requirement: 1,
        };
        &CAPS
    }

    async fn allocate_unified(
        &self,
        _size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        Err(ToadStoolError::runtime(
            "OpenCL unified memory removed (S198): use barraCuda/coralReef via IPC.",
        ))
    }

    async fn free_unified(&self, _allocation: BackendAllocation) -> ToadStoolResult<()> {
        Err(ToadStoolError::runtime(
            "OpenCL unified memory removed (S198): use barraCuda/coralReef via IPC.",
        ))
    }

    async fn map_cpu_ptr(&self, _allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        Err(ToadStoolError::runtime(
            "OpenCL unified memory removed (S198): use barraCuda/coralReef via IPC.",
        ))
    }

    fn get_device_ptr(&self, _allocation: &BackendAllocation) -> *const u8 {
        std::ptr::null()
    }
}
