// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL Backend — **DEPRECATED S198**
//!
//! Direct `ocl` FFI has been removed. OpenCL-style dispatch is now handled by
//! **barraCuda** and **coralReef** via capability-based IPC.
//!
//! ## Migration
//!
//! ```ignore
//! // OLD: direct ocl FFI
//! let backend = OpenClBackend::new()?;
//!
//! // NEW: capability-based IPC
//! let gpu = discover_capability("gpu.dispatch.opencl").await?;
//! gpu.call("gpu.dispatch", kernel_request).await?;
//! ```

use std::sync::Arc;

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Stub — OpenCL backend removed S198; use barraCuda/coralReef via IPC.
#[deprecated(
    since = "0.1.0",
    note = "ocl removed S198. OpenCL dispatch is handled by barraCuda/coralReef via IPC."
)]
pub struct OpenClBackend {
    _private: (),
}

#[expect(deprecated)]
impl OpenClBackend {
    /// Always returns an error directing callers to barraCuda/coralReef.
    #[deprecated(
        since = "0.1.0",
        note = "OpenClBackend is a stub (S198); use barraCuda/coralReef via capability IPC."
    )]
    pub fn new() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "OpenClBackend removed (S198): use barraCuda or coralReef via capability-based IPC \
             for OpenCL-class dispatch. See ecosystem GPU primals.",
        ))
    }
}

/// Stub — OpenCL compute resource removed S198; use barraCuda/coralReef via IPC.
#[deprecated(
    since = "0.1.0",
    note = "ocl removed S198. OpenCL dispatch is handled by barraCuda/coralReef via IPC."
)]
pub struct OpenClComputeResource {
    _backend: Arc<()>,
}
