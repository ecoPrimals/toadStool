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
use std::sync::Arc;

pub mod akida;
pub mod akida_executor;
pub mod async_submit;
pub mod autotune;
pub mod cache_hierarchy;
pub mod capabilities;
pub mod compute_pipeline;
pub mod driver_profile; // GPU driver/compiler identity + shader strategies (D-S17-002 refactor)
pub mod kernel_router; // Unified Math → Hardware routing (Feb 15, 2026)
pub mod latency; // LatencyModel trait + Sm70/Rdna2/AppleM/Conservative/Measured (SOVEREIGN Phase 2, Feb 2026)
pub mod pipeline_cache;
pub mod probe; // Runtime f64 exp/log capability probing (W-001 evolution)
pub mod probe_throughput; // f64 throughput ratio probing (metalForge discovery)
pub mod registry; // Physical device tracking with backend preference (Feb 16, 2026)
pub mod substrate;
pub mod tensor_context;
pub mod toadstool_integration;
pub mod tpu;
pub mod unified;
pub mod vendor; // Canonical GPU vendor ID constants (single source of truth)
pub mod warmup;
pub mod wgpu_device;

// Re-export auto-tuning types
pub use autotune::{AutoTuner, GpuCalibration, GLOBAL_TUNER};

// Re-export warmup (mise en place)
pub use warmup::{
    warmup_device, warmup_pool, WarmupConfig, WarmupOp, WarmupResult, WarmupWorkloadHint,
};

// Re-export pipeline cache (for testing cache clearing)
pub use pipeline_cache::clear_global_cache;

// Re-export tensor context (zero-overhead Tensor operations)
pub use tensor_context::{
    clear_global_contexts, get_device_context, high_capacity_limits, science_limits, BufferPool,
    PooledBuffer, TensorContext, TensorContextStats, TensorSession,
};

pub use akida::{detect_akida_boards, AkidaBoard, AkidaCapabilities, BoardHealth};
pub use akida_executor::{AkidaExecutor, NeuromorphicComparison};
pub use async_submit::{AsyncReadback, AsyncSubmitter};
pub use cache_hierarchy::{
    CacheAwareTiler, CacheLevel, CacheResidency, MainMemory, SubstrateMemoryHierarchy, TileConfig,
};
pub use capabilities::{
    optimal_workgroup_size_arch, workgroup_size_2d_for_arch, workgroup_size_for_arch, CompilerKind,
    DeviceCapabilities, DriverKind, EigensolveStrategy, Fp64Rate, Fp64Strategy, GpuArch,
    GpuDriverProfile, Workaround, WorkloadType,
};
pub use compute_pipeline::{storage_bgl_entry, uniform_bgl_entry, ComputeDispatch};
pub use kernel_router::{ComputeWorkload, KernelRouter, KernelTarget, NpuModelInfo};
pub use registry::{
    BackendInfo, DeviceCapabilities as PhysicalDeviceCapabilities, DeviceRegistry, DeviceVendor,
    PhysicalDevice, PhysicalDeviceId,
};
pub use substrate::{Substrate, SubstrateCapability, SubstrateType};
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
    /// Returns shared `WgpuDevice` from the global pool for thread-safe concurrent access.
    /// This enables parallel tests and concurrent GPU workloads without resource exhaustion.
    ///
    /// **Architecture**: Uses LazyLock-based pool (Rust 1.80+) for idiomatic lazy initialization.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() -> Result<Arc<WgpuDevice>> {
        Ok(test_pool::get_test_device().await)
    }

    /// Create a fresh device (not from pool)
    ///
    /// Use sparingly - creates a new device each call, which can exhaust GPU resources.
    /// Prefer `Auto::new()` for most cases.
    pub async fn new_fresh() -> Result<WgpuDevice> {
        WgpuDevice::new().await
    }
}
