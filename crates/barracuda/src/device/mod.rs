//! Device module - Pure WGSL via WebGPU
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders only (no separate CPU code!)
//! - wgpu handles CPU/GPU/NPU/TPU automatically
//! - Single implementation per operation
//! - Hardware-agnostic via WebGPU

use crate::error::Result;

pub mod wgpu_device;
pub mod akida;
pub mod akida_executor;

pub use wgpu_device::WgpuDevice;
pub use akida::{AkidaBoard, AkidaCapabilities, BoardHealth, detect_akida_boards};
pub use akida_executor::{AkidaExecutor, NeuromorphicComparison};

#[cfg(test)]
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
    pub async fn new() -> Result<WgpuDevice> {
        WgpuDevice::new().await
    }
}
