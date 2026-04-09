// SPDX-License-Identifier: AGPL-3.0-or-later
//! **Deprecated** — OpenCL is not implemented in the universal runtime.
//!
//! In-process GPU compute uses **wgpu** (Vulkan, Metal, Direct3D 12). OpenCL-style
//! dispatch is handled out-of-tree via **barraCuda** / **coralReef** over IPC.
//!
//! The [`OpenClComputeUnit`] name is kept only for migration visibility; it cannot
//! be used for real execution.

use crate::types::*;
use std::sync::OnceLock;

fn legacy_opencl_capabilities() -> &'static Capabilities {
    static CAPS: OnceLock<Capabilities> = OnceLock::new();
    CAPS.get_or_init(|| {
        let tag = u32::from_le_bytes(*b"OCL\0");
        Capabilities {
            unit_type: ComputeUnitType::Custom(tag),
            parallelism: Parallelism {
                num_units: 0,
                model: ExecutionModel::Mimd,
            },
            power_profile: PowerProfile::Medium,
            latency: LatencyProfile {
                typical_ms: 0,
                deterministic: false,
            },
            memory_capacity: 0,
            memory_bandwidth: 0,
            compute_throughput: 0.0,
            optimal_batch_size: 0,
            supported_ops: vec![],
            supported_types: vec![],
        }
    })
}

/// Legacy OpenCL compute unit placeholder (non-functional).
#[deprecated(
    since = "0.2.0",
    note = "OpenCL was removed from this crate; use wgpu/Vulkan or barraCuda/coralReef IPC."
)]
pub struct OpenClComputeUnit {
    _private: (),
}

#[expect(deprecated)]
impl OpenClComputeUnit {
    /// Always fails — OpenCL is not available in this runtime.
    pub fn new() -> Result<Self, ComputeError> {
        Err(ComputeError::BackendError(
            "OpenCL is not available in toadstool-runtime-universal. \
             Use wgpu for Vulkan/Metal/DX12, or barraCuda/coralReef for IPC OpenCL dispatch."
                .to_string(),
        ))
    }
}

#[expect(deprecated)]
#[async_trait::async_trait]
impl ComputeUnit for OpenClComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        legacy_opencl_capabilities()
    }

    fn name(&self) -> &'static str {
        "OpenCL (deprecated)"
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        Err(ComputeError::BackendError(
            "OpenCL is not available in toadstool-runtime-universal. \
             Use wgpu or barraCuda/coralReef IPC."
                .to_string(),
        ))
    }
}
