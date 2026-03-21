// SPDX-License-Identifier: AGPL-3.0-only
//! wgpu compute unit implementation (pure Rust!)
//!
//! This shows how wgpu GPUs are treated as ComputeUnits.
//! Key advantage: Pure Rust, no FFI!

mod initialization;
#[cfg(test)]
mod tests;
mod types;

use crate::types::*;
use std::sync::Arc;

pub use types::{
    GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, PrecisionRoutingAdvice,
    SubstrateCapabilityKind,
};

/// wgpu compute unit — hardware discovery layer for GPU adapters.
///
/// toadStool discovers and exposes adapter identity and limits so that
/// barraCuda (compute math primal) can make driver-aware decisions
/// (NVK detection, f64 workarounds, workgroup tuning).
pub struct WgpuComputeUnit {
    name: String,
    capabilities: Capabilities,
    adapter_info: GpuAdapterInfo,
    _adapter: wgpu::Adapter,
    _device: Arc<wgpu::Device>,
    _queue: Arc<wgpu::Queue>,
}

impl WgpuComputeUnit {
    /// Get the adapter identity info for driver-aware decisions.
    ///
    /// barraCuda reads this to build its `GpuDriverProfile` (NVK detection,
    /// f64 workarounds, workgroup tuning) without depending on wgpu.
    #[must_use]
    pub const fn adapter_info(&self) -> &GpuAdapterInfo {
        &self.adapter_info
    }
}

#[async_trait::async_trait]
impl ComputeUnit for WgpuComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        // toadStool provides hardware discovery and capability probing.
        // GPU compute dispatch (shaders, pipelines) is barraCuda's domain.
        // Use barraCuda's ComputeDispatch for actual GPU execution.
        Err(ComputeError::ExecutionFailed(
            "GPU compute dispatch is barraCuda's domain — discover via 'compute' capability IPC"
                .to_string(),
        ))
    }
}
