//! OpenCL compute unit implementation (DEPRECATED)
//!
//! **STATUS**: ⚠️ **DEPRECATED** - Use `wgpu` backend instead
//! **REASON**: wgpu provides better Rust ergonomics and broader hardware support
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
//! ## Deep Debt Evolution
//!
//! **Before**: OpenCL with C FFI, unsafe bindings  
//! **After**: Pure Rust wgpu with zero unsafe  
//! **Benefit**: Memory safety, better errors, faster compilation
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
    /// **If you need GPU compute**, use:
    /// ```rust
    /// use barracuda::device::WgpuDevice;
    /// let device = WgpuDevice::new().await?;
    /// ```
    ///
    /// Returns clear error directing users to wgpu
    #[deprecated(since = "0.2.0", note = "Use barracuda::device::WgpuDevice")]
    pub fn from_device(_device: ocl::Device) -> Result<Self, ComputeError> {
        Err(ComputeError::BackendError(anyhow::anyhow!(
            "OpenCL backend is deprecated. Use barracuda::device::WgpuDevice for GPU compute (pure Rust, safer, faster)"
        )))

        // OLD CODE - needs updating for new ocl API:
        // let name = device.name().map_err(|e| ComputeError::BackendError(e.into()))?;
        // ✅ Note: OpenCL is deprecated in favor of WGPU/barraCUDA
        // Keeping minimal impl for backward compatibility during transition
        // let max_compute_units = device.info(DeviceInfo::MaxComputeUnits)?;
        // let global_mem_size = device.info(DeviceInfo::GlobalMemSize)?;
    }
}

#[async_trait::async_trait]
impl ComputeUnit for OpenClComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        // Deep Debt Evolution: Clear error message with migration path
        Err(ComputeError::ExecutionFailed(
            "OpenCL backend is deprecated. Migrate to wgpu backend for GPU compute. \
             See docs/architecture/UNIVERSAL_GPU_STRATEGY.md for migration guide.".to_string(),
        ))
    }
}
