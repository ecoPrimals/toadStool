//! OpenCL compute unit implementation (legacy/stub)
//!
//! **STATUS**: Stub - OpenCL API needs modernization for new ocl crate
//! **RECOMMENDED**: Use wgpu (pure Rust) as primary GPU path
//!
//! This shows how OpenCL GPUs are treated as ComputeUnits.
//! The ocl crate API has changed significantly:
//! - Platform::list() now returns Vec<Platform> directly (not Result)
//! - Device info methods have changed (use info() with specific InfoKinds)
//! - Need to update all device queries to match new API
//!
//! For production use, prioritize wgpu (backends/wgpu_backend.rs) which:
//! - Is pure Rust (no FFI, no unsafe in application code)
//! - Works on NVIDIA, AMD, Intel via Vulkan/Metal/DX12
//! - Has been verified and is production-ready

use crate::types::*;

/// OpenCL compute unit (stub)
pub struct OpenClComputeUnit {
    name: String,
    capabilities: Capabilities,
    _device: ocl::Device,
}

impl OpenClComputeUnit {
    /// Create from an OpenCL device
    ///
    /// **TODO**: Update for new ocl crate API
    /// - Use device.info(DeviceInfo::MaxComputeUnits)
    /// - Use device.info(DeviceInfo::GlobalMemSize)
    /// - Handle the new Result/Option return patterns
    pub fn from_device(_device: ocl::Device) -> Result<Self, ComputeError> {
        // Temporary stub - returns error until API is modernized
        // Use wgpu backend for production GPU compute
        Err(ComputeError::BackendError(anyhow::anyhow!(
            "OpenCL backend needs API modernization - use wgpu instead"
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
        // Placeholder - full implementation would use ocl crate
        Err(ComputeError::ExecutionFailed(
            "OpenCL execution not yet fully implemented in universal runtime".to_string(),
        ))
    }
}
