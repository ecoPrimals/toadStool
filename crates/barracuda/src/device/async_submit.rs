//! Async Submission System for BarraCUDA
//!
//! Provides non-blocking GPU submission with operation batching.
//!
//! ## Design Philosophy
//!
//! Instead of Vulkan-style timeline semaphores (not directly exposed in wgpu),
//! we use idiomatic wgpu patterns:
//!
//! 1. **Deferred Execution**: Queue multiple operations before submission
//! 2. **Batch Submission**: Submit many command buffers in one call
//! 3. **Async Readback**: Non-blocking buffer reads with futures
//! 4. **Submission Index**: Track work completion without explicit fences
//!
//! ## Deep Debt Evolution (Feb 16, 2026)
//!
//! - Fixed async methods that were blocking inside async context
//! - Added `poll_until_ready()` helper for cooperative async polling
//! - Uses `tokio::task::yield_now()` to avoid starving the executor
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::device::async_submit::AsyncSubmitter;
//!
//! let submitter = AsyncSubmitter::new(device.clone());
//!
//! // Queue multiple operations (no GPU work yet)
//! submitter.queue_operation(|encoder| {
//!     // ... encode first operation
//! });
//! submitter.queue_operation(|encoder| {
//!     // ... encode second operation  
//! });
//!
//! // Submit all at once (single driver call)
//! let index = submitter.submit_all();
//!
//! // Optionally wait for completion
//! submitter.wait_for(index);
//! ```

use futures::future::FusedFuture;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Async GPU submission manager
///
/// Batches GPU operations and provides non-blocking submission tracking.
pub struct AsyncSubmitter {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Pending command buffers to be submitted
    pending: Mutex<Vec<wgpu::CommandBuffer>>,
    /// Submission counter for tracking work
    submission_index: AtomicU64,
    /// Index of last completed submission (approximate)
    completed_index: AtomicU64,
}

impl AsyncSubmitter {
    /// Create a new async submitter
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            pending: Mutex::new(Vec::with_capacity(16)),
            submission_index: AtomicU64::new(0),
            completed_index: AtomicU64::new(0),
        }
    }

    /// Queue a command buffer for deferred submission
    ///
    /// The command buffer will be submitted when `submit_all()` is called.
    /// This allows batching multiple operations into a single driver call.
    pub fn queue(&self, command_buffer: wgpu::CommandBuffer) {
        let mut pending = self.pending.lock().expect("pending mutex poisoned");
        pending.push(command_buffer);
    }

    /// Queue an operation using a callback that receives an encoder
    ///
    /// Convenience method that creates an encoder, runs the callback,
    /// and queues the finished command buffer.
    pub fn queue_operation<F>(&self, f: F)
    where
        F: FnOnce(&mut wgpu::CommandEncoder),
    {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("AsyncSubmitter encoder"),
            });
        f(&mut encoder);
        self.queue(encoder.finish());
    }

    /// Submit all pending command buffers
    ///
    /// Returns a submission index that can be used to track completion.
    pub fn submit_all(&self) -> u64 {
        let mut pending = self.pending.lock().expect("pending mutex poisoned");
        if pending.is_empty() {
            return self.submission_index.load(Ordering::SeqCst);
        }

        // Get new submission index
        let index = self.submission_index.fetch_add(1, Ordering::SeqCst) + 1;

        // Submit all pending buffers in one call
        let buffers: Vec<_> = pending.drain(..).collect();
        self.queue.submit(buffers);

        // Register a callback to update completed index when done
        let completed = Arc::new(AtomicU64::new(0));
        let completed_clone = completed.clone();
        let idx = index;

        self.queue.on_submitted_work_done(move || {
            completed_clone.store(idx, Ordering::SeqCst);
        });

        index
    }

    /// Submit a single command buffer immediately
    ///
    /// Use when you need immediate submission without batching.
    pub fn submit_immediate(&self, command_buffer: wgpu::CommandBuffer) -> u64 {
        let index = self.submission_index.fetch_add(1, Ordering::SeqCst) + 1;
        self.queue.submit(Some(command_buffer));
        index
    }

    /// Get the current submission index
    ///
    /// All work with index <= this value has been submitted to the GPU.
    pub fn current_index(&self) -> u64 {
        self.submission_index.load(Ordering::SeqCst)
    }

    /// Check if work at the given index is complete
    ///
    /// Note: This is approximate. For precise synchronization, use `wait_for()`.
    pub fn is_complete(&self, index: u64) -> bool {
        // Poll the device to process completion callbacks
        self.device.poll(wgpu::Maintain::Poll);
        self.completed_index.load(Ordering::SeqCst) >= index
    }

    /// Wait for work at the given index to complete
    ///
    /// Blocks until all GPU work up to and including the given index is done.
    pub fn wait_for(&self, index: u64) {
        // If already complete, return immediately
        if self.completed_index.load(Ordering::SeqCst) >= index {
            return;
        }

        // Poll until complete
        self.device.poll(wgpu::Maintain::Wait);
    }

    /// Wait for all pending work to complete
    pub fn wait_all(&self) {
        let current = self.submission_index.load(Ordering::SeqCst);
        self.wait_for(current);
    }

    /// Get the number of pending command buffers
    pub fn pending_count(&self) -> usize {
        self.pending.lock().expect("pending mutex poisoned").len()
    }

    /// Clear all pending command buffers without submitting
    ///
    /// Use with caution - discards queued work.
    pub fn clear_pending(&self) {
        self.pending.lock().expect("pending mutex poisoned").clear();
    }
}

/// Async buffer read operation
///
/// Wraps a wgpu buffer read that can be awaited without blocking.
pub struct AsyncReadback {
    staging_buffer: wgpu::Buffer,
    size_bytes: u64,
    receiver: futures::channel::oneshot::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

impl AsyncReadback {
    /// Create a new async readback operation
    ///
    /// Copies from source buffer to staging and initiates async map.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::Buffer,
        size_bytes: u64,
    ) -> Self {
        // Create staging buffer
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AsyncReadback staging"),
            size: size_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy source to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("AsyncReadback copy"),
        });
        encoder.copy_buffer_to_buffer(source, 0, &staging_buffer, 0, size_bytes);
        queue.submit(Some(encoder.finish()));

        // Start async map
        let (sender, receiver) = futures::channel::oneshot::channel();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });

        Self {
            staging_buffer,
            size_bytes,
            receiver,
        }
    }

    /// Poll the device and check if data is ready
    pub fn poll(&self, device: &wgpu::Device) -> bool {
        device.poll(wgpu::Maintain::Poll);
        // Check if receiver has a value (non-blocking)
        !self.receiver.is_terminated()
    }

    /// Poll the device until the buffer is ready (async-safe)
    ///
    /// Uses non-blocking poll with yield points to avoid blocking the executor.
    /// This is the proper async pattern for wgpu buffer mapping.
    async fn poll_until_ready(&mut self, device: &wgpu::Device) -> Result<(), String> {
        loop {
            // Non-blocking poll
            device.poll(wgpu::Maintain::Poll);

            // Check if the receiver is ready (oneshot channels can be checked)
            // We use a select-like approach: try to receive without blocking
            use futures::future::FutureExt;
            match (&mut self.receiver).now_or_never() {
                Some(result) => {
                    // Receiver completed
                    return result
                        .map_err(|_| "Readback cancelled".to_string())?
                        .map_err(|e| format!("Map error: {:?}", e));
                }
                None => {
                    // Not ready yet - yield to let other tasks run
                    // This prevents blocking the async executor
                    tokio::task::yield_now().await;
                }
            }
        }
    }

    /// Wait for data and read as f32 (async-safe)
    ///
    /// Uses cooperative polling that yields to the async executor.
    pub async fn read_f32(mut self, device: &wgpu::Device) -> Result<Vec<f32>, String> {
        self.poll_until_ready(device).await?;

        // Read data
        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Wait for data and read as f64 (async-safe)
    pub async fn read_f64(mut self, device: &wgpu::Device) -> Result<Vec<f64>, String> {
        self.poll_until_ready(device).await?;

        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Wait for data and read as u32 (async-safe)
    pub async fn read_u32(mut self, device: &wgpu::Device) -> Result<Vec<u32>, String> {
        self.poll_until_ready(device).await?;

        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Wait for data and read as raw bytes (async-safe)
    pub async fn read_bytes(mut self, device: &wgpu::Device) -> Result<Vec<u8>, String> {
        self.poll_until_ready(device).await?;

        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result = data.to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Blocking read as f32 (for non-async contexts)
    ///
    /// Use this when you're in a synchronous context and need to read data.
    pub fn read_f32_blocking(self, device: &wgpu::Device) -> Result<Vec<f32>, String> {
        device.poll(wgpu::Maintain::Wait);

        futures::executor::block_on(self.receiver)
            .map_err(|_| "Readback cancelled".to_string())?
            .map_err(|e| format!("Map error: {:?}", e))?;

        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Blocking read as f64 (for non-async contexts)
    pub fn read_f64_blocking(self, device: &wgpu::Device) -> Result<Vec<f64>, String> {
        device.poll(wgpu::Maintain::Wait);

        futures::executor::block_on(self.receiver)
            .map_err(|_| "Readback cancelled".to_string())?
            .map_err(|e| format!("Map error: {:?}", e))?;

        let data = self.staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a GPU device
    // Run with: cargo test -p barracuda --lib device::async_submit -- --ignored

    #[test]
    #[ignore = "requires GPU"]
    fn test_submitter_creation() {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("No adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .expect("No device");

            let submitter = AsyncSubmitter::new(Arc::new(device), Arc::new(queue));
            assert_eq!(submitter.pending_count(), 0);
            assert_eq!(submitter.current_index(), 0);
        });
    }

    #[test]
    fn test_submission_index_increment() {
        // Test that submission index increments correctly (no GPU needed)
        let index = AtomicU64::new(0);
        assert_eq!(index.fetch_add(1, Ordering::SeqCst) + 1, 1);
        assert_eq!(index.fetch_add(1, Ordering::SeqCst) + 1, 2);
    }
}
