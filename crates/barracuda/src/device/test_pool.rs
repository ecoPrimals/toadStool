//! Test device pool - reuse devices across tests to prevent exhaustion
//!
//! **Problem**: Creating 272 wgpu devices exhausts GPU resources
//! **Solution**: Shared device pool with lazy initialization
//! **Deep Debt**: Runtime discovery, no hardcoding, thread-safe
//! **Evolution**: Migrated from once_cell to std::sync::LazyLock (Rust 1.80+)

use crate::device::WgpuDevice;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

/// Global device pool for tests
///
/// **Deep Debt Principles**:
/// - Runtime discovery (no hardcoded device)
/// - Thread-safe (Arc + Mutex)
/// - Lazy initialization (only create when needed)
/// - Reusable (shared across all tests)
/// - Pure std (no external lazy_static or once_cell)
static TEST_DEVICE_POOL: LazyLock<Arc<Mutex<Option<Arc<WgpuDevice>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Get or create shared test device
///
/// **Usage in tests**:
/// ```rust
/// #[tokio::test]
/// async fn test_matmul() {
///     let Some(dev) = get_test_device().await else { return };
///     let result = matmul(&dev.device, &dev.queue, ...).await.unwrap();
/// }
/// ```
///
/// **Benefits**:
/// - Fixes 119 test failures (device exhaustion)
/// - Faster tests (reuse device initialization)
/// - Thread-safe (multiple tests can share)
pub async fn get_test_device() -> Arc<WgpuDevice> {
    let mut pool = TEST_DEVICE_POOL.lock().await;

    if let Some(device) = pool.as_ref() {
        // Reuse existing device
        return Arc::clone(device);
    }

    // Create new device (first test only)
    let device = Arc::new(
        WgpuDevice::new()
            .await
            .expect("Failed to create test device"),
    );

    *pool = Some(Arc::clone(&device));
    device
}

/// Reset device pool (for integration tests that need fresh state)
///
/// **Use sparingly**: Only when tests need isolated devices
pub async fn reset_test_device_pool() {
    let mut pool = TEST_DEVICE_POOL.lock().await;
    *pool = None;
}

/// GPU-only device pool for tests that require real GPU hardware.
///
/// Software adapters (llvmpipe, lavapipe, swiftshader) produce NaN/Inf for
/// transcendental operations. Tests using this pool skip when no real GPU exists.
static TEST_GPU_DEVICE_POOL: LazyLock<Arc<Mutex<Option<Arc<WgpuDevice>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Get test device, returning None if only a software/CPU adapter is available.
///
/// GPU shader tests should use this to gracefully skip on machines without GPUs.
/// Use: `let Some(device) = get_test_device().await else { return };`
pub async fn get_test_device_if_gpu_available() -> Option<Arc<WgpuDevice>> {
    let mut pool = TEST_GPU_DEVICE_POOL.lock().await;

    if let Some(device) = pool.as_ref() {
        return Some(Arc::clone(device));
    }

    match WgpuDevice::new_gpu().await {
        Ok(device) => {
            let device = Arc::new(device);
            *pool = Some(Arc::clone(&device));
            Some(device)
        }
        Err(_) => None,
    }
}

/// f64-capable GPU device pool for tests requiring SHADER_F64 feature.
///
/// Many scientific shaders (SSF, forces, PDE solvers) require f64 precision.
/// This pool selects a GPU with wgpu::Features::SHADER_F64 enabled.
static TEST_F64_GPU_DEVICE_POOL: LazyLock<Arc<Mutex<Option<Arc<WgpuDevice>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Get f64-capable test device, returning None if no f64 GPU is available.
///
/// f64 shader tests should use this to gracefully skip on machines without f64 GPUs.
/// Use: `let Some(device) = get_test_device_if_f64_gpu_available().await else { return };`
pub async fn get_test_device_if_f64_gpu_available() -> Option<Arc<WgpuDevice>> {
    let mut pool = TEST_F64_GPU_DEVICE_POOL.lock().await;

    if let Some(device) = pool.as_ref() {
        return Some(Arc::clone(device));
    }

    match WgpuDevice::new_f64_capable().await {
        Ok(device) => {
            let device = Arc::new(device);
            *pool = Some(Arc::clone(&device));
            Some(device)
        }
        Err(_) => None,
    }
}

// ============================================================================
// Sync helpers - always available for test modules across crate
// ============================================================================

/// Sync wrapper for `get_test_device`.
///
/// **Prefer async**: Use `get_test_device().await` in `#[tokio::test]` when possible.
/// This sync helper exists for test functions that can't be async.
///
/// **Thread-safe**: Multiple tests can call this concurrently — they all get the same device.
pub fn get_test_device_sync() -> Arc<WgpuDevice> {
    pollster::block_on(get_test_device())
}

/// Sync wrapper for `get_test_device_if_gpu_available`.
///
/// Returns `None` if only a software adapter is available. Use for tests requiring real GPU.
pub fn get_test_device_if_gpu_available_sync() -> Option<Arc<WgpuDevice>> {
    pollster::block_on(get_test_device_if_gpu_available())
}

/// Sync wrapper for `get_test_device_if_f64_gpu_available`.
///
/// Returns `None` if no f64-capable GPU is present. Use for double-precision shader tests.
pub fn get_test_device_if_f64_gpu_available_sync() -> Option<Arc<WgpuDevice>> {
    pollster::block_on(get_test_device_if_f64_gpu_available())
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

        // Box-Muller for normal distribution
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
        // First access creates device
        let dev1 = get_test_device().await;
        let ptr1 = Arc::as_ptr(&dev1);

        // Second access reuses device
        let dev2 = get_test_device().await;
        let ptr2 = Arc::as_ptr(&dev2);

        // Should be same device (same pointer)
        assert_eq!(ptr1, ptr2, "Device pool should reuse same device");
    }

    #[tokio::test]
    async fn test_device_pool_concurrent() {
        // Multiple concurrent accesses should all get same device
        let handles: Vec<_> = (0..10).map(|_| tokio::spawn(get_test_device())).collect();

        let mut devices = Vec::with_capacity(handles.len());
        for h in handles {
            devices.push(h.await.unwrap());
        }

        // All should point to same device
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
