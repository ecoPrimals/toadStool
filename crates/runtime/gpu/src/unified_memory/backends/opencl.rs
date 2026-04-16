// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL unified memory backend — **DEPRECATED S198**
//!
//! Direct OpenCL SVM via `ocl` has been removed. Use **barraCuda** / **coralReef**
//! for GPU memory and dispatch via IPC.

use crate::unified_memory::backend::{BackendAllocation, BackendInitializer, UnifiedMemoryBackend};
use crate::unified_memory::types::{BackendType, MemoryFlags, UnifiedMemoryCapabilities};
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Documentary alias: stub [`UnifiedMemoryCapabilities`] values for the removed OpenCL backend (S198).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL S198 stub marker type; use barraCuda/coralReef for real capability data."
)]
pub type OpenClUnifiedMemoryStubCapabilities = UnifiedMemoryCapabilities;

/// All-zero/false stub returned by the deprecated OpenCL unified-memory backend (S198).
///
/// **Not** populated from hardware or ICD probing — legacy placeholder only.
#[deprecated(
    since = "0.1.0",
    note = "OpenCL S198 stub static; all-zero fields are placeholders, not device discovery."
)]
pub static OPENCL_UNIFIED_MEMORY_STUB_CAPS: UnifiedMemoryCapabilities = UnifiedMemoryCapabilities {
    backend_type: BackendType::OpenCL,
    max_allocation_size: 0,
    zero_copy: false,
    coherent: false,
    cpu_fast_access: false,
    gpu_fast_access: false,
    alignment_requirement: 1,
};

/// Stub backend retained for API compatibility (initialization always fails).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL unified memory removed S198; use barraCuda/coralReef via IPC."
)]
pub struct OpenClBackend {
    _private: (),
}

/// Compile-time stub: in-tree OpenCL unified memory is never available (S198).
#[inline]
pub const fn opencl_unified_memory_never_available() -> bool {
    false
}

#[expect(deprecated)]
impl BackendInitializer for OpenClBackend {
    /// Always fails — OpenCL unified memory was removed (S198).
    async fn try_init() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "OpenCL unified memory removed (S198): use barraCuda/coralReef via IPC.",
        ))
    }

    /// **Stub:** always `false` — not runtime probing; the OpenCL path is removed (S198).
    fn is_available() -> bool {
        opencl_unified_memory_never_available()
    }
}

#[expect(deprecated)]
impl UnifiedMemoryBackend for OpenClBackend {
    fn name(&self) -> &'static str {
        "OpenCL (deprecated)"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::OpenCL
    }

    /// Returns the deprecated stub capability record (all zeros/false); not hardware discovery.
    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        #[expect(deprecated)]
        {
            &OPENCL_UNIFIED_MEMORY_STUB_CAPS
        }
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
