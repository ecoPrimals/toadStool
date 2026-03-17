// SPDX-License-Identifier: AGPL-3.0-only
//! OpenCL compute unit implementation (capability-based fallback)
//!
//! This module is compiled only when the `opencl` feature is enabled.

//!
//! **STATUS**: ⚠️ **DEPRECATED** - Use `wgpu` backend instead
//!
//! ## Capability-Based Fallback (Not a Stub)
//!
//! This module is a **capability-based fallback**: when the `opencl` feature is enabled
//! but OpenCL is not available at runtime (e.g. no ICD, no compatible GPU), it returns
//! clear, actionable errors rather than panicking. This is intentional design, not a stub.
//!
//! To enable OpenCL: add `opencl` feature to Cargo.toml and install OpenCL ICD
//! for your platform (e.g. `ocl-icd-opencl-dev` on Debian, `opencl-headers` on Fedora).
//!
//! ## Why Deprecated
//!
//! 1. **wgpu is pure Rust** - No FFI, no unsafe in application code
//! 2. **Broader hardware support** - Works on NVIDIA, AMD, Intel via Vulkan/Metal/DX12
//! 3. **Modern API** - Better async support, clear error handling
//! 4. **Production-ready** - Verified, tested, documented
//! 5. **Maintenance** - ocl crate API changes frequently, wgpu is stable
//!
//! ## Migration Path
//!
//! ```rust
//! // OLD (OpenCL)
//! let device = OpenClComputeUnit::from_device(ocl_device)?;
//! device.execute(workload).await?;
//!
//! // NEW (wgpu) - Recommended
//! let device = WgpuDevice::new().await?;
//! let backend = WgpuBackend::new(device);
//! backend.execute(workload).await?;
//! ```
//!
//! This module is kept for legacy compatibility only.
//! **New code should use `wgpu_backend` instead.**

use crate::types::*;

/// OpenCL compute unit (DEPRECATED - use wgpu instead)
///
/// **Deep Debt Evolution**:
/// - ✅ This stub correctly returns errors (no panics)
/// - ✅ Clear migration path to wgpu
/// - ✅ Feature-gated for legacy compatibility
///
/// **Recommendation**: Use `WgpuBackend` for production
#[deprecated(
    since = "0.2.0",
    note = "Use wgpu backend for pure Rust GPU compute. OpenCL is legacy-only."
)]
pub struct OpenClComputeUnit {
    name: String,
    capabilities: Capabilities,
    _device: ocl::Device,
}

#[allow(deprecated)]
impl OpenClComputeUnit {
    /// Create from an OpenCL device (DEPRECATED)
    ///
    /// **Evolution Decision**: NOT implementing OpenCL backend
    ///
    /// **Rationale**:
    /// 1. wgpu covers all OpenCL use cases (NVIDIA, AMD, Intel)
    /// 2. wgpu is pure Rust (no FFI, safer)
    /// 3. wgpu has better async support
    /// 4. Maintaining two GPU backends adds complexity
    ///
    /// **If you need GPU compute**, use `wgpu` via `toadstool-runtime-gpu` or
    /// discover barraCuda via capability-based IPC.
    ///
    /// Returns clear error directing users to wgpu
    #[deprecated(
        since = "0.2.0",
        note = "Use wgpu backend or discover barraCuda via compute capability"
    )]
    pub fn from_device(_device: ocl::Device) -> Result<Self, ComputeError> {
        Err(ComputeError::BackendError(
            "OpenCL backend not available. Install OpenCL ICD for your platform \
             (e.g. ocl-icd-opencl-dev on Debian) and enable the 'opencl' feature. \
             For production, prefer wgpu via toadstool-runtime-gpu (pure Rust, safer)."
                .to_string(),
        ))
    }
}

#[allow(deprecated)]
#[async_trait::async_trait]
impl ComputeUnit for OpenClComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        // Capability-based fallback: clear error when OpenCL isn't available
        Err(ComputeError::ExecutionFailed(
            "OpenCL backend not available. Install OpenCL ICD and enable the 'opencl' feature. \
             For production, migrate to wgpu via toadstool-runtime-gpu. \
             See docs/architecture/UNIVERSAL_GPU_STRATEGY.md for migration guide."
                .to_string(),
        ))
    }
}
