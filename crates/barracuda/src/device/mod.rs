//! Device module - Unified Hardware Abstraction
//!
//! **Phase 2: Unified Device Architecture**:
//! - Single Device enum for ALL hardware
//! - Automatic device selection
//! - Explicit routing when needed
//! - Flexible fallback chains
//! - Runtime capability discovery
//!
//! **Hardware Types**:
//! - CPU: Pure Rust execution
//! - GPU: WGSL shaders via wgpu
//! - NPU: Akida neuromorphic
//! - TPU: Tensor Processing Units (future)
//! - Auto: Smart selection

use crate::error::Result;

pub mod akida;
pub mod akida_executor;
pub mod autotune;
pub mod capabilities;
pub mod pipeline_cache;
pub mod substrate;
pub mod tensor_context;
pub mod toadstool_integration; // NEW: ToadStool hardware layer integration
pub mod tpu;
pub mod unified;
pub mod warmup;
pub mod wgpu_device;

// Re-export auto-tuning types
pub use autotune::{AutoTuner, GpuCalibration, GLOBAL_TUNER};

// Re-export warmup (mise en place)
pub use warmup::{warmup_device, warmup_pool, WarmupConfig, WarmupOp, WarmupResult, WarmupWorkloadHint};

// Re-export tensor context (zero-overhead Tensor operations)
pub use tensor_context::{
    get_device_context, clear_global_contexts,
    BufferPool, TensorContext, TensorContextStats, high_capacity_limits
};

pub use akida::{detect_akida_boards, AkidaBoard, AkidaCapabilities, BoardHealth};
pub use akida_executor::{AkidaExecutor, NeuromorphicComparison};
pub use capabilities::{DeviceCapabilities, WorkloadType};
pub use substrate::{Substrate, SubstrateType};
pub use toadstool_integration::{
    discover_devices, hardware_report, has_gpu, has_npu, select_best_device, select_device_prefer,
    DeviceSelection, HardwareReport, HardwareWorkload,
};
pub use tpu::{TpuDevice, TpuGeneration, TpuInfo};
pub use unified::{Capability, Device, DeviceContext, DeviceInfo, WorkloadHint};
pub use wgpu_device::WgpuDevice;

/// Device pool for GPU operations (used by NMS and tests).
/// Always compiled so NMS can acquire a GPU device at runtime.
pub mod test_pool;

/// Auto device discovery via wgpu
///
/// wgpu automatically handles:
/// - GPU (Vulkan, Metal, DX12) - preferred
/// - CPU (software rasterizer) - automatic fallback
/// - NPU/TPU (if wgpu driver available)
pub struct Auto;

impl Auto {
    /// Discover best available device (wgpu handles selection)
    ///
    /// Returns `WgpuDevice` (not `Self`) because `Auto` is a zero-sized factory type.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() -> Result<WgpuDevice> {
        WgpuDevice::new().await
    }
}
