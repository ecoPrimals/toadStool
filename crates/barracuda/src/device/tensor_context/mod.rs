//! TensorContext - Zero-overhead Tensor operations via internal pooling

mod context;
mod limits;
mod pool;

pub use context::{clear_global_contexts, get_device_context, TensorContext, TensorContextStats};
pub use limits::{high_capacity_limits, science_limits};
pub use pool::{BufferDescriptor, BufferPool, PooledBuffer, SolverBufferSet};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    use std::sync::Arc;

    async fn create_test_device() -> Arc<wgpu::Device> {
        Arc::clone(&get_test_device().await.device)
    }

    #[tokio::test]
    async fn test_buffer_pool_basic_acquire_release() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        let buf1 = pool.acquire(1024);
        assert!(buf1.size() >= 1024);
        let size = buf1.size();
        pool.release(buf1);
        let buf2 = pool.acquire(1024);
        assert_eq!(buf2.size(), size);
        let (allocs, reuses) = pool.stats();
        assert_eq!(allocs, 1);
        assert_eq!(reuses, 1);
    }

    #[tokio::test]
    async fn test_buffer_pool_power_of_two_bucketing() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        assert_eq!(pool.acquire(1000).size(), 1024);
        assert_eq!(pool.acquire(1025).size(), 2048);
        assert_eq!(pool.acquire(100).size(), 256);
    }

    #[tokio::test]
    async fn test_buffer_pool_multiple_buckets() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        let buf_256 = pool.acquire(256);
        let buf_1024 = pool.acquire(1024);
        let buf_4096 = pool.acquire(4096);
        pool.release(buf_256);
        pool.release(buf_1024);
        pool.release(buf_4096);
        assert_eq!(pool.acquire(200).size(), 256);
        assert_eq!(pool.acquire(1000).size(), 1024);
        assert_eq!(pool.acquire(4000).size(), 4096);
    }

    #[tokio::test]
    async fn test_pooled_buffer_auto_return() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        let (allocs_before, reuses_before) = pool.stats();
        {
            let _ = pool.acquire_pooled(1024);
        }
        let _ = pool.acquire_pooled(1024);
        let (allocs_after, reuses_after) = pool.stats();
        assert_eq!(allocs_after - allocs_before, 1);
        assert_eq!(reuses_after - reuses_before, 1);
    }

    #[tokio::test]
    async fn test_tensor_context_acquire_pooled() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);
        assert!(ctx.acquire_pooled_output(1000).size() >= 4000);
    }

    #[tokio::test]
    async fn test_tensor_context_batching_mode() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);
        assert!(!ctx.is_batching());
        ctx.begin_batch();
        assert!(ctx.is_batching());
        ctx.end_batch().expect("end_batch failed");
        assert!(!ctx.is_batching());
    }

    #[tokio::test]
    async fn test_tensor_context_stats() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);
        let stats = ctx.stats();
        assert_eq!(stats.buffer_allocations, 0);
        assert_eq!(stats.ops_executed, 0);
    }

    #[tokio::test]
    async fn test_global_context_registry() {
        let device = get_test_device().await;
        let ctx1 = get_device_context(&device);
        let ctx2 = get_device_context(&device);
        let initial = ctx1.stats().buffer_allocations;
        let _ = ctx1.acquire_pooled_output(1000);
        assert!(ctx2.stats().buffer_allocations >= initial);
    }

    #[test]
    fn test_high_capacity_limits() {
        let limits = high_capacity_limits();
        assert_eq!(limits.max_storage_buffer_binding_size, 1 << 30);
        assert_eq!(limits.max_buffer_size, 1 << 31);
    }

    #[tokio::test]
    async fn test_pin_solver_buffers_basic() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        let buffers = pool
            .pin_solver_buffers(
                "test_solver",
                &[
                    ("matrix_a", BufferDescriptor::f64_array(100)),
                    ("result", BufferDescriptor::f64_array(100)),
                ],
            )
            .expect("pin failed");
        assert_eq!(buffers.len(), 2);
        assert!(buffers.get("matrix_a").is_some());
        assert!(pool.release_solver_buffers("test_solver"));
    }

    #[tokio::test]
    async fn test_pin_solver_buffers_duplicate_id_error() {
        let device = create_test_device().await;
        let pool = BufferPool::new(device);
        pool.pin_solver_buffers("solver_a", &[("buf", BufferDescriptor::new(256))])
            .expect("first pin");
        assert!(pool
            .pin_solver_buffers("solver_a", &[("buf2", BufferDescriptor::new(256))])
            .is_err());
        pool.release_solver_buffers("solver_a");
        assert!(pool
            .pin_solver_buffers("solver_a", &[("buf2", BufferDescriptor::new(256))])
            .is_ok());
    }

    #[tokio::test]
    async fn test_buffer_descriptor_helpers() {
        assert_eq!(BufferDescriptor::f64_array(100).size, 800);
        assert_eq!(BufferDescriptor::f32_array(100).size, 400);
        assert_eq!(
            BufferDescriptor::new(1024).with_label("x").label,
            Some("x".to_string())
        );
    }

    #[tokio::test]
    async fn test_tensor_context_solver_buffers() {
        let device = get_test_device().await;
        let ctx = TensorContext::new(device);
        let buffers = ctx
            .pin_solver_buffers(
                "scf",
                &[
                    ("hamiltonian", BufferDescriptor::f64_array(64)),
                    ("density", BufferDescriptor::f64_array(32)),
                ],
            )
            .expect("pin");
        assert_eq!(buffers.len(), 2);
        assert!(ctx.get_solver_buffers("scf").is_some());
        assert!(ctx.release_solver_buffers("scf"));
        assert!(ctx.get_solver_buffers("scf").is_none());
    }
}
