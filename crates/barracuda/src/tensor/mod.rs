//! Unified Tensor abstraction — hardware-agnostic tensor compute.
//!
//! - Single `Tensor` type works on any device
//! - Self-knowledge: every `Tensor` carries its `WgpuDevice`
//! - Operations dispatch automatically to the owning device
//! - Zero duplication across backends
//! - Buffer pooling for zero-allocation steady state

pub(crate) mod buffer;
mod ops;
use buffer::TensorBuffer;

use crate::device::tensor_context::PooledBuffer;
use crate::device::{Auto, Device, WgpuDevice, WorkloadHint};
use crate::error::{BarracudaError, Result};
use std::sync::Arc;

/// Tensor - hardware-agnostic tensor via WGSL/WebGPU
///
/// **Philosophy**:
/// - Works seamlessly on GPU (via WGSL) or CPU (via Rayon)
/// - Auto-discovers best device when created
/// - Operations execute on tensor's device automatically
/// - Explicit device transfer when needed
///
/// **Deep Debt Excellence**:
/// - Zero-copy reshape via `Arc<Buffer>` sharing
/// - Safe Rust (no unsafe needed)
/// - Fast (metadata-only operations)
/// - Pooled buffers for zero-allocation steady state
///
/// ## Examples
///
/// ```rust,ignore
/// use barracuda::prelude::*;
///
/// // Auto-discovers best device (GPU if available)
/// let x = Tensor::zeros([128, 256])?;
///
/// // Operations execute on same device
/// let y = x.relu()?;
/// let z = y.softmax(0)?;
///
/// println!("Executed on: {}", x.device().name());
/// ```
pub struct Tensor {
    /// GPU buffer - either owned or pooled
    /// (pooled buffers automatically return to pool on drop)
    buffer: TensorBuffer,

    /// Tensor shape (dimensions)
    shape: Vec<usize>,

    /// Device (WebGPU - works everywhere!)
    device: Arc<WgpuDevice>,

    /// Optional name (for debugging)
    name: Option<String>,
}

impl Tensor {
    /// Create tensor from an existing `wgpu::Buffer`.
    ///
    /// This constructor lets callers build GPU-resident pipelines that skip the
    /// GPU→CPU→GPU round-trip: dispatch a compute shader, then wrap its output
    /// buffer directly into a `Tensor` without ever reading back to the CPU.
    ///
    /// # Example
    /// ```ignore
    /// // After dispatching a custom compute shader:
    /// let t = Tensor::from_buffer(output_buffer, shape, device);
    /// // `t` stays fully GPU-resident — no readback.
    /// ```
    pub fn from_buffer(buffer: wgpu::Buffer, shape: Vec<usize>, device: Arc<WgpuDevice>) -> Self {
        Self {
            buffer: TensorBuffer::Owned(Arc::new(buffer)),
            shape,
            device,
            name: None,
        }
    }

    /// Create a tensor that **shares** an `Arc<wgpu::Buffer>` with the caller.
    ///
    /// This is the zero-copy path for bridge code (e.g. `GpuExecutor`) that
    /// already holds an `Arc<wgpu::Buffer>` and wants to wrap it as a `Tensor`
    /// without a GPU→CPU→GPU round-trip.
    ///
    /// The buffer's lifetime is shared: both the `Tensor` and the caller's `Arc`
    /// keep it alive.
    pub fn from_arc_buffer(
        buffer: Arc<wgpu::Buffer>,
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Self {
        Self {
            buffer: TensorBuffer::Owned(buffer),
            shape,
            device,
            name: None,
        }
    }

    /// Return the underlying `Arc<wgpu::Buffer>` if this tensor owns an unshared
    /// (non-pooled) buffer, otherwise `None`.
    ///
    /// Callers that need direct buffer access for zero-copy bridge code use this
    /// to avoid copying buffer contents.
    pub fn try_arc_buffer(&self) -> Option<Arc<wgpu::Buffer>> {
        self.buffer.try_arc()
    }

    /// Create tensor from pooled buffer (internal use)
    ///
    /// Pooled buffers automatically return to their pool when the
    /// tensor is dropped, enabling zero-allocation steady state.
    pub(crate) fn from_pooled_buffer(
        buffer: PooledBuffer,
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Self {
        Self {
            buffer: TensorBuffer::Pooled(Arc::new(buffer)),
            shape,
            device,
            name: None,
        }
    }

    /// Get reference to buffer (internal use)
    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.as_ref()
    }

    /// Check if this tensor uses a pooled buffer
    pub fn is_pooled(&self) -> bool {
        matches!(self.buffer, TensorBuffer::Pooled(_))
    }

    /// Query which unified Device type this tensor is on.
    ///
    /// Maps the runtime `wgpu::DeviceType` to the canonical `Device` enum.
    pub fn query_device(&self) -> Device {
        // All tensors use WgpuDevice; map hardware type to Device variant.
        match self.device.device_type() {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => Device::GPU,
            wgpu::DeviceType::VirtualGpu => Device::GPU,
            wgpu::DeviceType::Cpu => Device::CPU,
            wgpu::DeviceType::Other => Device::Auto,
        }
    }

    /// Create a routing preference for this tensor's operations
    ///
    /// Record a routing preference; live tensor migration is deferred (D-S18-003).
    ///
    /// # Example
    /// ```ignore
    /// let tensor = Tensor::randn(vec![1000, 1000]).await?;
    /// let gpu_tensor = tensor.prefer_device(Device::GPU);
    /// ```
    pub fn prefer_device(&self, _device: Device) -> Self {
        // Routing hint recorded; live device migration deferred (tracked as D-S18-003).
        log::debug!("Device preference noted; migration deferred (D-S18-003)");
        self.clone()
    }

    /// Create tensor with workload hint for smart routing.
    ///
    /// # Example
    /// ```ignore
    /// let tensor = Tensor::randn(vec![100, 100]).await?
    ///     .with_hint(WorkloadHint::SmallWorkload); // Prefers CPU
    /// ```
    #[must_use]
    pub fn with_hint(&self, hint: WorkloadHint) -> Self {
        let preferred_device = Device::select_for_workload(&hint);
        log::debug!("Workload hint: {:?} → Device: {}", hint, preferred_device);
        self.clone()
    }

    /// Create tensor from f32 data (primary API for testing and initialization)
    ///
    /// Accepts `&[f32]` explicitly to prevent accidental f64 type inference
    /// when using bare float literals like `&[1.0, 2.0, 3.0]`.
    pub fn from_data(data: &[f32], shape: Vec<usize>, device: Arc<WgpuDevice>) -> Result<Self> {
        Self::from_data_pod(data, shape, device)
    }

    /// Create tensor from arbitrary Pod data (for FHE/u32 workloads)
    ///
    /// Use `from_data` for f32 tensors. This method exists for operations
    /// that need non-f32 buffer data (e.g., FHE polynomial operations using u32).
    ///
    /// **Note**: `to_vec()` always reads back as f32. For u32 data, use `to_vec_u32()`.
    pub fn from_data_pod<T: bytemuck::Pod>(
        data: &[T],
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;

        // Generate unique label to avoid buffer caching/collision
        let label = format!(
            "Tensor Data {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        let buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        Ok(Self {
            buffer: TensorBuffer::Owned(Arc::new(buffer)),
            shape,
            device,
            name: None,
        })
    }

    /// Create tensor from `Vec<f32>` data (convenience method for operations)
    ///
    /// This is used by WGSL operations to return computed results.
    pub fn new(data: Vec<f32>, shape: Vec<usize>, device: Arc<WgpuDevice>) -> Self {
        use wgpu::util::DeviceExt;

        // Handle empty data: create a minimum-size buffer for WebGPU compatibility
        let buffer = if data.is_empty() {
            device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Tensor Empty"),
                size: 4, // Minimum 4 bytes
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        } else {
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Tensor"),
                    contents: bytemuck::cast_slice(&data),
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                })
        };

        Self {
            buffer: TensorBuffer::Owned(Arc::new(buffer)),
            shape,
            device,
            name: None,
        }
    }
}

// Implement Clone for Tensor
impl Clone for Tensor {
    fn clone(&self) -> Self {
        // **Zero-Copy Clone**: Arc makes this cheap!
        // Both tensors share the same GPU buffer memory.
        Self {
            buffer: self.buffer.clone(), // Arc clone - just increments reference count
            shape: self.shape.clone(),
            device: self.device.clone(),
            name: self.name.clone(),
        }
    }
}

impl Tensor {
    /// Deep clone - creates a new buffer with copied data
    ///
    /// Use this when you need independent buffers.
    /// Regular `.clone()` is zero-copy (shared buffer).
    pub fn deep_clone(&self) -> Result<Self> {
        let size = self.len();
        let new_buffer = self.device.create_buffer_f32(size)?;

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Tensor Deep Clone Encoder"),
                });

        encoder.copy_buffer_to_buffer(
            self.buffer(),
            0,
            &new_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.device.queue.submit(Some(encoder.finish()));

        Ok(Self {
            buffer: TensorBuffer::Owned(Arc::new(new_buffer)),
            shape: self.shape.clone(),
            device: self.device.clone(),
            name: self.name.clone(),
        })
    }
}

impl Tensor {
    /// Create tensor from data (wgpu auto-discovers best device)
    pub async fn from_vec(data: Vec<f32>, shape: Vec<usize>) -> Result<Self> {
        let device = Auto::new().await?;
        Self::from_vec_on(data, shape, device).await
    }

    /// Create tensor on specific device
    pub async fn from_vec_on(
        data: Vec<f32>,
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Result<Self> {
        // Delegate to sync version - no actual async needed
        Self::from_vec_on_sync(data, shape, device)
    }

    /// Create tensor on specific device (synchronous version)
    ///
    /// Use this when you already have a device reference and need to create
    /// a tensor without async context (e.g., in scalar operations).
    pub fn from_vec_on_sync(
        data: Vec<f32>,
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Result<Self> {
        // Validate shape
        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::shape_mismatch(
                vec![expected_size],
                vec![data.len()],
            ));
        }

        // Create buffer and write data
        let buffer = device.create_buffer_f32(data.len())?;
        device.write_buffer_f32(&buffer, &data)?;

        Ok(Self {
            buffer: TensorBuffer::Owned(Arc::new(buffer)),
            shape,
            device,
            name: None,
        })
    }

    /// Create zero tensor (wgpu auto-discovers device)
    pub async fn zeros(shape: Vec<usize>) -> Result<Self> {
        let device = Auto::new().await?;
        Self::zeros_on(shape, device).await
    }

    /// Create zero tensor on specific device
    pub async fn zeros_on(shape: Vec<usize>, device: Arc<WgpuDevice>) -> Result<Self> {
        let size: usize = shape.iter().product();
        let data = vec![0.0; size];
        Self::from_vec_on(data, shape, device).await
    }

    /// Create ones tensor
    pub async fn ones(shape: Vec<usize>) -> Result<Self> {
        let device = Auto::new().await?;
        Self::ones_on(shape, device).await
    }

    /// Create ones tensor on specific device
    pub async fn ones_on(shape: Vec<usize>, device: Arc<WgpuDevice>) -> Result<Self> {
        let size: usize = shape.iter().product();
        let data = vec![1.0; size];
        Self::from_vec_on(data, shape, device).await
    }

    /// Get tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Get number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Is tensor empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get device this tensor lives on
    pub fn device(&self) -> &Arc<WgpuDevice> {
        &self.device
    }

    /// Set tensor name (for debugging)
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get tensor name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Read tensor data to host memory
    pub fn to_vec(&self) -> Result<Vec<f32>> {
        self.device.read_buffer_f32(self.buffer(), self.len())
    }

    /// Read tensor data as u32 (for FHE operations using u64 as u32 pairs)
    pub fn to_vec_u32(&self) -> Result<Vec<u32>> {
        self.device.read_buffer_u32(self.buffer(), self.len())
    }

    /// Read tensor data as f64 (for high-precision operations)
    ///
    /// Used by FFT f64, sparse solvers, and PPPM integration.
    /// Note: Buffer size is interpreted as f64 element count (8 bytes each).
    pub fn to_f64_vec(&self) -> Result<Vec<f64>> {
        self.device.read_buffer_f64(self.buffer(), self.len())
    }

    /// Create tensor from f64 data
    ///
    /// Used for high-precision operations (PPPM, FFT f64, sparse solvers).
    pub fn from_f64_data(data: &[f64], shape: Vec<usize>, device: Arc<WgpuDevice>) -> Result<Self> {
        Self::from_data_pod(data, shape, device)
    }

    /// Transfer tensor to another device
    pub async fn to_device(&self, target_device: Arc<WgpuDevice>) -> Result<Self> {
        let data = self.to_vec()?;
        Self::from_vec_on(data, self.shape.clone(), target_device).await
    }

    // Scalar ops (mul_scalar, add_scalar, div_scalar) and random generation
    // (randn, rand, rand_range, etc.) are in tensor/ops.rs.

    /// Reshape tensor (zero-copy via Arc buffer sharing)
    ///
    /// **Deep Debt Excellence**:
    /// - True zero-copy: shares same GPU buffer via Arc
    /// - Just metadata change (shape update)
    /// - Fast AND safe (no unsafe code needed)
    /// - Modern idiomatic Rust (Arc for shared ownership)
    ///
    /// ## Example
    /// ```rust,ignore
    /// let x = Tensor::zeros([2, 3, 4]).await?;  // [2, 3, 4]
    /// let y = x.reshape([6, 4])?;                // [6, 4] - same buffer!
    /// ```
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Self> {
        // Validate element count matches
        let old_size: usize = self.shape.iter().product();
        let new_size: usize = new_shape.iter().product();

        if old_size != new_size {
            return Err(BarracudaError::shape_mismatch(
                vec![new_size],
                vec![old_size],
            ));
        }

        // **Zero-Copy Implementation**: wgpu buffers are always contiguous,
        // so reshape is always safe and zero-copy - we just update metadata!
        //
        // The Arc<Buffer> is cloned (cheap ref count increment), not the buffer.
        // Both tensors share the same GPU memory.
        //
        // This is safe because:
        // 1. Element count is validated (old_size == new_size)
        // 2. wgpu buffers are always contiguous (no striding issues)
        // 3. Arc provides safe shared ownership
        // 4. No unsafe code needed!
        Ok(Self {
            buffer: self.buffer.clone(), // Arc clone - zero-copy!
            shape: new_shape,
            device: self.device.clone(),
            name: self.name.clone(),
        })
    }
}

impl std::fmt::Debug for Tensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tensor")
            .field("shape", &self.shape)
            .field("device", &self.device.name())
            .field("name", &self.name)
            .field("len", &self.len())
            .finish()
    }
}

impl std::fmt::Display for Tensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tensor{:?} on {} ({})",
            self.shape,
            self.device.name(),
            if let Some(name) = &self.name {
                name.as_str()
            } else {
                "unnamed"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tensor_creation() {
        let tensor = Tensor::zeros(vec![2, 3]).await.unwrap();
        assert_eq!(tensor.shape(), &[2, 3]);
        assert_eq!(tensor.len(), 6);

        let data = tensor.to_vec().unwrap();
        assert_eq!(data, vec![0.0; 6]);
    }

    #[tokio::test]
    async fn test_tensor_from_vec() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let tensor = Tensor::from_vec(data.clone(), vec![2, 3]).await.unwrap();

        assert_eq!(tensor.shape(), &[2, 3]);
        assert_eq!(tensor.to_vec().unwrap(), data);
    }

    #[tokio::test]
    async fn test_tensor_reshape() {
        let tensor = Tensor::ones(vec![2, 3]).await.unwrap();
        let reshaped = tensor.reshape(vec![3, 2]).unwrap();

        assert_eq!(reshaped.shape(), &[3, 2]);
        assert_eq!(reshaped.len(), 6);
    }

    #[tokio::test]
    async fn test_tensor_device() {
        let tensor = Tensor::zeros(vec![10]).await.unwrap();
        println!("Tensor on device: {}", tensor.device().name());
        assert!(!tensor.device().name().is_empty());
    }

    #[tokio::test]
    async fn test_scalar_mul() {
        let tensor = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], vec![4])
            .await
            .unwrap();
        let result = tensor.mul_scalar(2.0).unwrap();
        let data = result.to_vec().unwrap();

        assert_eq!(data, vec![2.0, 4.0, 6.0, 8.0]);
    }

    #[tokio::test]
    async fn test_scalar_add() {
        let tensor = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], vec![4])
            .await
            .unwrap();
        let result = tensor.add_scalar(10.0).unwrap();
        let data = result.to_vec().unwrap();

        assert_eq!(data, vec![11.0, 12.0, 13.0, 14.0]);
    }

    #[tokio::test]
    async fn test_scalar_div() {
        let tensor = Tensor::from_vec(vec![10.0, 20.0, 30.0, 40.0], vec![4])
            .await
            .unwrap();
        let result = tensor.div_scalar(2.0).unwrap();
        let data = result.to_vec().unwrap();

        assert_eq!(data, vec![5.0, 10.0, 15.0, 20.0]);
    }

    #[tokio::test]
    async fn test_randn_shape() {
        let tensor = Tensor::randn(vec![10, 20]).await.unwrap();
        assert_eq!(tensor.shape(), &[10, 20]);
        assert_eq!(tensor.len(), 200);

        // Check values are reasonable for N(0,1)
        let data = tensor.to_vec().unwrap();
        let mean: f32 = data.iter().sum::<f32>() / data.len() as f32;
        let variance: f32 =
            data.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / data.len() as f32;

        // Mean should be close to 0, std close to 1
        assert!(mean.abs() < 0.3, "Mean {} too far from 0", mean);
        assert!(
            (variance.sqrt() - 1.0).abs() < 0.3,
            "Std {} too far from 1",
            variance.sqrt()
        );
    }

    #[tokio::test]
    async fn test_rand_shape() {
        let tensor = Tensor::rand(vec![10, 20]).await.unwrap();
        assert_eq!(tensor.shape(), &[10, 20]);
        assert_eq!(tensor.len(), 200);

        // Check values are in [0, 1)
        let data = tensor.to_vec().unwrap();
        for &val in &data {
            assert!((0.0..1.0).contains(&val), "Value {} out of range", val);
        }
    }

    #[tokio::test]
    async fn test_rand_range() {
        let tensor = Tensor::rand_range(vec![100], -5.0, 5.0).await.unwrap();
        let data = tensor.to_vec().unwrap();

        // Check all values in range
        for &val in &data {
            assert!((-5.0..5.0).contains(&val), "Value {} out of range", val);
        }

        // Mean should be near 0
        let mean: f32 = data.iter().sum::<f32>() / data.len() as f32;
        assert!(mean.abs() < 1.0, "Mean {} too far from 0", mean);
    }

    #[tokio::test]
    async fn test_randn_reproducible() {
        use rand::SeedableRng;

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let tensor1 = Tensor::randn_with_rng(vec![10], &mut rng1).await.unwrap();

        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let tensor2 = Tensor::randn_with_rng(vec![10], &mut rng2).await.unwrap();

        assert_eq!(tensor1.to_vec().unwrap(), tensor2.to_vec().unwrap());
    }

    #[tokio::test]
    async fn test_query_device() {
        let tensor = Tensor::randn(vec![10]).await.unwrap();
        let device = tensor.query_device();

        // Device should be one of the valid types
        assert!(matches!(device, Device::CPU | Device::GPU | Device::Auto));
    }

    #[tokio::test]
    async fn test_prefer_device() {
        let tensor = Tensor::randn(vec![10, 10]).await.unwrap();

        // Test device preference (Phase 2: just sets hint)
        let gpu_tensor = tensor.prefer_device(Device::GPU);
        assert_eq!(gpu_tensor.shape(), tensor.shape());
        assert_eq!(gpu_tensor.len(), tensor.len());
    }

    #[tokio::test]
    async fn test_with_hint() {
        let tensor = Tensor::randn(vec![5, 5]).await.unwrap();

        // Test workload hints
        let small = tensor.with_hint(WorkloadHint::SmallWorkload);
        assert_eq!(small.shape(), tensor.shape());

        let large = tensor.with_hint(WorkloadHint::LargeMatrices);
        assert_eq!(large.shape(), tensor.shape());
    }

    #[tokio::test]
    async fn test_tensor_3d_roundtrip_minimal() -> Result<()> {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return Ok(());
        };

        println!("\n=== Testing 3D Tensor Roundtrip ===\n");

        // Test various 3D shapes
        for &(nx, ny, nz) in &[(2, 2, 2), (3, 3, 3), (4, 4, 4)] {
            let size = nx * ny * nz;
            println!("Testing shape [{nx}, {ny}, {nz}] (size={size}):");

            // Create data: all 1.0s
            let data = vec![1.0f32; size];
            println!("  Input: {} elements, all 1.0", data.len());

            // Create tensor
            let tensor = Tensor::from_data(&data, vec![nx, ny, nz], device.clone()).unwrap();
            println!(
                "  Tensor created: shape={:?}, len={}",
                tensor.shape(),
                tensor.len()
            );

            // Read back
            let result = tensor.to_vec().unwrap();
            println!("  Output: {} elements", result.len());
            println!("  First 10 values: {:?}", &result[0..10.min(result.len())]);

            // Validate
            assert_eq!(result.len(), size, "Length mismatch");

            let mut errors = 0;
            for (i, &val) in result.iter().enumerate() {
                if (val - 1.0).abs() > 1e-5 {
                    if errors < 5 {
                        println!("  ❌ Index {i}: expected 1.0, got {val}");
                    }
                    errors += 1;
                }
            }

            if errors == 0 {
                println!("  ✅ All values correct!\n");
            } else {
                println!("  ❌ {} corrupted values out of {}\n", errors, size);
                return Err(crate::error::BarracudaError::Internal(format!(
                    "Tensor 3D corruption detected for shape [{nx}, {ny}, {nz}]"
                )));
            }
        }

        println!("=== All 3D shapes passed! ===");
        Ok(())
    }

    #[tokio::test]
    async fn test_tensor_laplacian_context_debug() -> Result<()> {
        // EXACT COPY of test_tensor_3d_roundtrip_minimal
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return Ok(());
        };

        println!("\n=== Testing 3D Tensor Roundtrip (Laplacian Context) ===\n");

        // Test just 3x3x3 (the failing case)
        let (nx, ny, nz) = (3, 3, 3);
        let size = nx * ny * nz;
        println!("Testing shape [{nx}, {ny}, {nz}] (size={size}):");

        // Create data: all 1.0s
        let data = vec![1.0f32; size];
        println!("  Input: {} elements, all 1.0", data.len());

        // Create tensor
        let tensor = Tensor::from_data(&data, vec![nx, ny, nz], device.clone()).unwrap();
        println!(
            "  Tensor created: shape={:?}, len={}",
            tensor.shape(),
            tensor.len()
        );

        // Read back
        let result = tensor.to_vec().unwrap();
        println!("  Output: {} elements", result.len());
        println!("  First 10 values: {:?}", &result[0..10.min(result.len())]);

        // Validate
        assert_eq!(result.len(), size, "Length mismatch");

        let mut errors = 0;
        for (i, &val) in result.iter().enumerate() {
            if (val - 1.0).abs() > 1e-5 {
                if errors < 5 {
                    println!("  ❌ Index {i}: expected 1.0, got {val}");
                }
                errors += 1;
            }
        }

        if errors == 0 {
            println!("  ✅ All values correct!\n");
        } else {
            println!("  ❌ {} corrupted values out of {}\n", errors, size);
            return Err(crate::error::BarracudaError::Internal(format!(
                "Tensor 3D corruption detected for shape [{nx}, {ny}, {nz}]"
            )));
        }

        println!("=== Test passed! ===");
        Ok(())
    }
}
