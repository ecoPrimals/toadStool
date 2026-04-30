// SPDX-License-Identifier: AGPL-3.0-or-later
//! CUDA Backend — **DEPRECATED S197**
//!
//! Direct `cudarc` FFI has been removed. CUDA dispatch is now handled by
//! capability providers discovered at runtime via `gpu.dispatch.cuda`.
//!
//! ToadStool discovers CUDA capability at runtime through the ecosystem's
//! coordination mesh rather than embedding the NVIDIA toolchain.
//!
//! ## Migration
//!
//! ```ignore
//! // OLD: direct cudarc FFI
//! let backend = CudaBackend::new()?;
//!
//! // NEW: capability-based IPC
//! let cuda = discover_capability("gpu.dispatch.cuda").await?;
//! cuda.call("gpu.dispatch", kernel_request).await?;
//! ```

use std::sync::Arc;

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Stub — CUDA backend removed S197; discover `gpu.dispatch.cuda` capability at runtime.
#[deprecated(
    since = "0.1.0",
    note = "cudarc removed S197. Discover `gpu.dispatch.cuda` capability provider via IPC."
)]
pub struct CudaBackend {
    _private: (),
}

#[expect(
    deprecated,
    reason = "stub impl kept for API compat; callers migrating to capability discovery"
)]
impl CudaBackend {
    /// Always returns an error directing callers to capability-based discovery.
    #[deprecated(
        since = "0.1.0",
        note = "CudaBackend is a stub (S197); discover `gpu.dispatch.cuda` capability via IPC."
    )]
    pub fn new() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "CudaBackend removed (S197): use `discover_capability(\"gpu.dispatch.cuda\")` \
             for CUDA dispatch via capability-based IPC.",
        ))
    }
}

/// Stub — CUDA compute resource removed S197; discover capability at runtime.
#[deprecated(
    since = "0.1.0",
    note = "cudarc removed S197. Discover `gpu.dispatch.cuda` capability provider via IPC."
)]
pub struct CudaComputeResource {
    _backend: Arc<()>,
}
