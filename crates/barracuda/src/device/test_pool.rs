//! Test device pool - reuse devices across tests to prevent exhaustion
//!
//! **Problem**: Creating 272 wgpu devices exhausts GPU resources
//! **Solution**: Shared device pool with lazy initialization
//! **Deep Debt**: Runtime discovery, no hardcoding, thread-safe

use crate::device::WgpuDevice;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Global device pool for tests
///
/// **Deep Debt Principles**:
/// - Runtime discovery (no hardcoded device)
/// - Thread-safe (Arc + Mutex)
/// - Lazy initialization (only create when needed)
/// - Reusable (shared across all tests)
static TEST_DEVICE_POOL: Lazy<Arc<Mutex<Option<Arc<WgpuDevice>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Get or create shared test device
///
/// **Usage in tests**:
/// ```rust
/// #[tokio::test]
/// async fn test_matmul() {
///     let dev = get_test_device().await;
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

        let devices: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

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
