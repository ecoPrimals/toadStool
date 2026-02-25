//! Test device pool — resilient shared device with automatic recovery
//!
//! **Problem**: Creating 2435 wgpu devices exhausts GPU resources.
//! **Problem**: A single NVVM/driver shader compilation failure marks the
//!   wgpu device as "lost", cascading to ALL subsequent tests.
//! **Problem**: Multi-GPU systems (e.g. RTX 3090 + RX 6950 XT) can silently
//!   select different adapters across recreations, causing cross-device
//!   contamination via `TensorContext` (keyed by adapter fingerprint).
//!
//! **Solution**: Shared device behind `RwLock` with health-check + recreation.
//!   ALL accessors funnel through ONE device. Adapter selection is pinned via
//!   `BARRACUDA_GPU_ADAPTER` / `HOTSPRING_GPU_ADAPTER` env vars, or auto-
//!   detected as the first discrete GPU with `SHADER_F64`.
//!
//! Absorbed from hotSpring adapter selection (Feb 2026).

use crate::device::WgpuDevice;
use std::sync::{Arc, RwLock};

static DEVICE_POOL: std::sync::LazyLock<RwLock<Option<Arc<WgpuDevice>>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

/// Cached adapter capabilities (survive device recreation).
static IS_REAL_GPU: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
static HAS_F64: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Pinned adapter selector — determined once, reused on every recreation so
/// the test pool always returns the same physical GPU.
static ADAPTER_SELECTOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn resolve_adapter_selector() -> String {
    if let Ok(v) = std::env::var("BARRACUDA_GPU_ADAPTER") {
        if !v.is_empty() {
            return v;
        }
    }
    if let Ok(v) = std::env::var("HOTSPRING_GPU_ADAPTER") {
        if !v.is_empty() {
            return v.split(',').next().unwrap_or("auto").to_string();
        }
    }
    // Auto-detect: find the first discrete GPU with SHADER_F64 and pin to its name.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::DiscreteGpu
            && adapter.features().contains(wgpu::Features::SHADER_F64)
        {
            log::info!("test_pool: auto-pinned to '{}' (discrete, f64)", info.name);
            return info.name.clone();
        }
    }
    "auto".to_string()
}

async fn create_device() -> Arc<WgpuDevice> {
    let selector = ADAPTER_SELECTOR.get_or_init(resolve_adapter_selector);
    let device = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        WgpuDevice::with_adapter_selector(selector),
    )
    .await
    .expect("GPU device creation timed out after 10s -- check driver")
    .expect("Failed to create test device");
    log::info!(
        "test_pool: device '{}' ({:?})",
        device.adapter_info().name,
        device.adapter_info().device_type,
    );
    Arc::new(device)
}

fn is_device_healthy(device: &WgpuDevice) -> bool {
    // A lost device panics in the uncaptured error handler on any GPU operation.
    // catch_unwind detects this without crashing the test runner.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let buf = device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("health-probe"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        device
            .queue()
            .submit(std::iter::empty::<wgpu::CommandBuffer>());
        device.device().poll(wgpu::Maintain::Wait);
        drop(buf);
    }))
    .is_ok()
}

/// Get or create the shared test device, recreating if the previous one was lost.
pub async fn get_test_device() -> Arc<WgpuDevice> {
    // Fast path: device exists and is healthy.
    {
        let guard = DEVICE_POOL.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ref dev) = *guard {
            if is_device_healthy(dev) {
                return Arc::clone(dev);
            }
        }
    }

    // Slow path: create or recreate.
    let mut guard = DEVICE_POOL.write().unwrap_or_else(|e| e.into_inner());
    // Double-check: another thread may have recreated while we waited for the write lock.
    if let Some(ref dev) = *guard {
        if is_device_healthy(dev) {
            return Arc::clone(dev);
        }
        log::warn!("Shared test device lost — recreating");
        crate::device::tensor_context::clear_global_contexts();
        crate::device::pipeline_cache::clear_global_cache();
    }

    let new_device = pollster::block_on(create_device());

    IS_REAL_GPU.get_or_init(|| new_device.adapter_info().device_type != wgpu::DeviceType::Cpu);
    HAS_F64.get_or_init(|| new_device.has_f64_shaders());

    *guard = Some(Arc::clone(&new_device));
    new_device
}

/// Get the shared device if it's a real GPU (not software/CPU adapter).
pub async fn get_test_device_if_gpu_available() -> Option<Arc<WgpuDevice>> {
    let device = get_test_device().await;
    if *IS_REAL_GPU.get_or_init(|| device.adapter_info().device_type != wgpu::DeviceType::Cpu) {
        Some(device)
    } else {
        None
    }
}

/// Get the shared device if it supports f64 shader operations.
pub async fn get_test_device_if_f64_gpu_available() -> Option<Arc<WgpuDevice>> {
    let device = get_test_device().await;
    if *HAS_F64.get_or_init(|| device.has_f64_shaders()) {
        Some(device)
    } else {
        None
    }
}

// ============================================================================
// Sync helpers
// ============================================================================

fn get_test_device_sync_inner() -> Arc<WgpuDevice> {
    // Try fast path without creating a runtime.
    {
        let guard = DEVICE_POOL.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ref dev) = *guard {
            if is_device_healthy(dev) {
                return Arc::clone(dev);
            }
        }
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime for sync device access")
        .block_on(get_test_device())
}

/// Run a closure with the shared test device.
pub fn run_with_sync_device<F, R>(f: F) -> R
where
    F: FnOnce(Arc<WgpuDevice>) -> R,
{
    f(get_test_device_sync())
}

/// Sync wrapper for `get_test_device`.
pub fn get_test_device_sync() -> Arc<WgpuDevice> {
    get_test_device_sync_inner()
}

/// Sync wrapper for `get_test_device_if_gpu_available`.
pub fn get_test_device_if_gpu_available_sync() -> Option<Arc<WgpuDevice>> {
    let device = get_test_device_sync();
    if *IS_REAL_GPU.get_or_init(|| device.adapter_info().device_type != wgpu::DeviceType::Cpu) {
        Some(device)
    } else {
        None
    }
}

/// Sync wrapper for `get_test_device_if_f64_gpu_available`.
pub fn get_test_device_if_f64_gpu_available_sync() -> Option<Arc<WgpuDevice>> {
    let device = get_test_device_sync();
    if *HAS_F64.get_or_init(|| device.has_f64_shaders()) {
        Some(device)
    } else {
        None
    }
}

// ============================================================================
// Test prelude - import this in test modules for easy device access
// ============================================================================

/// Test prelude for concurrent GPU tests
///
/// # Usage
/// ```rust,ignore
/// #[cfg(test)]
/// mod tests {
///     use crate::device::test_pool::test_prelude::*;
///     
///     #[tokio::test]
///     async fn test_my_op() {
///         let device = test_device().await;
///         let tensor = test_tensor(&[1.0, 2.0, 3.0], &[3], &device).await;
///         // ... test logic
///     }
/// }
/// ```
pub mod test_prelude {
    use super::*;
    use crate::tensor::Tensor;

    /// Get shared test device (async version - preferred)
    pub async fn test_device() -> Arc<WgpuDevice> {
        get_test_device().await
    }

    /// Get shared test device (sync version)
    pub fn test_device_blocking() -> Arc<WgpuDevice> {
        get_test_device_sync()
    }

    /// Get GPU-only test device, or skip test if unavailable
    pub async fn test_gpu_device() -> Option<Arc<WgpuDevice>> {
        get_test_device_if_gpu_available().await
    }

    /// Get f64-capable test device, or skip test if unavailable
    pub async fn test_f64_device() -> Option<Arc<WgpuDevice>> {
        get_test_device_if_f64_gpu_available().await
    }

    /// Create test tensor on shared device
    pub async fn test_tensor(data: &[f32], shape: &[usize], device: &Arc<WgpuDevice>) -> Tensor {
        Tensor::from_vec_on(data.to_vec(), shape.to_vec(), Arc::clone(device))
            .await
            .expect("Failed to create test tensor")
    }

    /// Create test tensor (sync version)
    pub fn test_tensor_blocking(data: &[f32], shape: &[usize], device: &Arc<WgpuDevice>) -> Tensor {
        pollster::block_on(test_tensor(data, shape, device))
    }

    /// Create zeros tensor on shared device
    pub async fn test_zeros(shape: &[usize], device: &Arc<WgpuDevice>) -> Tensor {
        Tensor::zeros_on(shape.to_vec(), Arc::clone(device))
            .await
            .expect("Failed to create zeros tensor")
    }

    /// Create randn tensor on shared device
    ///
    /// Uses Box-Muller transform on CPU, then uploads to shared device.
    pub async fn test_randn(shape: &[usize], device: &Arc<WgpuDevice>) -> Tensor {
        use rand::Rng;
        let size: usize = shape.iter().product();
        let mut rng = rand::thread_rng();

        let mut data = Vec::with_capacity(size);
        for _ in 0..(size / 2) {
            let u1: f32 = rng.gen::<f32>().max(1e-10);
            let u2: f32 = rng.gen();
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            data.push(r * theta.cos());
            data.push(r * theta.sin());
        }
        if size % 2 == 1 {
            let u1: f32 = rng.gen::<f32>().max(1e-10);
            let u2: f32 = rng.gen();
            data.push((-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos());
        }
        data.truncate(size);

        Tensor::from_vec_on(data, shape.to_vec(), Arc::clone(device))
            .await
            .expect("Failed to create randn tensor")
    }

    /// Create rand tensor on shared device (uniform [0, 1))
    pub async fn test_rand(shape: &[usize], device: &Arc<WgpuDevice>) -> Tensor {
        use rand::Rng;
        let size: usize = shape.iter().product();
        let data: Vec<f32> = (0..size).map(|_| rand::thread_rng().gen()).collect();

        Tensor::from_vec_on(data, shape.to_vec(), Arc::clone(device))
            .await
            .expect("Failed to create rand tensor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_pool_reuse() {
        let dev1 = get_test_device().await;
        let ptr1 = Arc::as_ptr(&dev1);

        let dev2 = get_test_device().await;
        let ptr2 = Arc::as_ptr(&dev2);

        assert_eq!(ptr1, ptr2, "Device pool should reuse same device");
    }

    #[tokio::test]
    async fn test_device_pool_concurrent() {
        let handles: Vec<_> = (0..10).map(|_| tokio::spawn(get_test_device())).collect();

        let mut devices = Vec::with_capacity(handles.len());
        for h in handles {
            devices.push(h.await.unwrap());
        }

        let first_ptr = Arc::as_ptr(&devices[0]);
        for dev in &devices[1..] {
            assert_eq!(
                Arc::as_ptr(dev),
                first_ptr,
                "Concurrent accesses should get same device"
            );
        }
    }
}
