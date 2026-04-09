// SPDX-License-Identifier: AGPL-3.0-or-later
//! CUDA Backend — **DEPRECATED S197**
//!
//! Direct `cudarc` FFI has been removed. CUDA dispatch is now handled by
//! **barraCuda** (PTX compilation, cuDNN, single-GPU) and **coralReef**
//! (multi-GPU orchestration) via capability-based IPC.
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

/// Stub — CUDA backend removed S197; use barraCuda/coralReef via IPC.
#[deprecated(
    since = "0.1.0",
    note = "cudarc removed S197. CUDA dispatch is handled by barraCuda/coralReef via IPC."
)]
pub struct CudaBackend {
    _private: (),
}

#[expect(deprecated)]
impl CudaBackend {
    /// Always returns an error directing callers to barraCuda/coralReef.
    pub fn new() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "CudaBackend removed (S197): use barraCuda or coralReef via capability-based IPC \
             for CUDA dispatch. See `discover_capability(\"gpu.dispatch.cuda\")`.",
        ))
    }
}

/// Stub — CUDA compute resource removed S197; use barraCuda/coralReef via IPC.
#[deprecated(
    since = "0.1.0",
    note = "cudarc removed S197. CUDA dispatch is handled by barraCuda/coralReef via IPC."
)]
pub struct CudaComputeResource {
    _backend: Arc<()>,
}
