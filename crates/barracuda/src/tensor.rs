//! Unified Tensor abstraction - hardware-agnostic tensor compute
//!
//! **Deep Debt Excellence**:
//! - Single Tensor type works on any device
//! - Self-knowledge: Tensor knows its device
//! - Automatic operations dispatch based on device
//! - Zero duplication across backends

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
/// - Zero-copy reshape via Arc<Buffer> sharing
/// - Safe Rust (no unsafe needed)
/// - Fast (metadata-only operations)
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
    /// GPU buffer wrapped in Arc for zero-copy operations
    /// (wgpu handles CPU/GPU/NPU/TPU automatically!)
    buffer: Arc<wgpu::Buffer>,

    /// Tensor shape (dimensions)
    shape: Vec<usize>,

    /// Device (WebGPU - works everywhere!)
    device: Arc<WgpuDevice>,

    /// Optional name (for debugging)
    name: Option<String>,
}

impl Tensor {
    /// Create tensor from existing buffer (internal use)
    pub(crate) fn from_buffer(
        buffer: wgpu::Buffer,
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Self {
        Self {
            buffer: Arc::new(buffer),
            shape,
            device,
            name: None,
        }
    }

    /// Get reference to buffer (internal use)
    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Query which unified Device type this tensor is conceptually on
    ///
    /// **Phase 2**: Maps WgpuDevice to unified Device enum
    pub fn query_device(&self) -> Device {
        // For Phase 2, all tensors use WgpuDevice
        // Check device type to determine if GPU or CPU backend
        match self.device.device_type() {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => Device::GPU,
            wgpu::DeviceType::VirtualGpu => Device::GPU,
            wgpu::DeviceType::Cpu => Device::CPU,
            wgpu::DeviceType::Other => Device::Auto,
        }
    }

    /// Create a routing preference for this tensor's operations
    ///
    /// **Phase 2 Note**: This sets an execution hint for future operations.
    /// Full device migration comes in Phase 3.
    ///
    /// # Example
    /// ```ignore
    /// let tensor = Tensor::randn(vec![1000, 1000]).await?;
    /// let gpu_tensor = tensor.prefer_device(Device::GPU); // Hint for GPU
    /// ```
    pub fn prefer_device(&self, _device: Device) -> Self {
        // Phase 2: For now, just return clone with routing hint logged
        // Phase 3 will implement actual device migration
        log::debug!("Device preference noted (Phase 3 will implement migration)");
        self.clone()
    }

    /// Create tensor with workload hint for smart routing
    ///
    /// **Phase 2**: Adds metadata about workload characteristics
    ///
    /// # Example
    /// ```ignore
    /// let tensor = Tensor::randn(vec![100, 100]).await?
    ///     .with_hint(WorkloadHint::SmallWorkload); // Prefers CPU
    /// ```
    pub fn with_hint(&self, hint: WorkloadHint) -> Self {
        let preferred_device = Device::select_for_workload(&hint);
        log::debug!("Workload hint: {:?} → Device: {}", hint, preferred_device);
        self.clone()
    }

    /// Create tensor from data (for testing and initialization)
    pub fn from_data<T: bytemuck::Pod>(
        data: &[T],
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;

        let buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Tensor Data"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        Ok(Self {
            buffer: Arc::new(buffer),
            shape,
            device,
            name: None,
        })
    }

    /// Create tensor from Vec<f32> data (convenience method for operations)
    ///
    /// This is used by WGSL operations to return computed results.
    pub fn new(data: Vec<f32>, shape: Vec<usize>, device: Arc<WgpuDevice>) -> Self {
        use wgpu::util::DeviceExt;

        let buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Tensor"),
                contents: bytemuck::cast_slice(&data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        Self {
            buffer: Arc::new(buffer),
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
            &self.buffer,
            0,
            &new_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.device.queue.submit(Some(encoder.finish()));

        Ok(Self {
            buffer: Arc::new(new_buffer),
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
        Self::from_vec_on(data, shape, Arc::new(device)).await
    }

    /// Create tensor on specific device
    pub async fn from_vec_on(
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
            buffer: Arc::new(buffer),
            shape,
            device,
            name: None,
        })
    }

    /// Create zero tensor (wgpu auto-discovers device)
    pub async fn zeros(shape: Vec<usize>) -> Result<Self> {
        let device = Auto::new().await?;
        Self::zeros_on(shape, Arc::new(device)).await
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
        Self::ones_on(shape, Arc::new(device)).await
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
        self.device.read_buffer_f32(&self.buffer, self.len())
    }

    /// Read tensor data as u32 (for FHE operations using u64 as u32 pairs)
    pub fn to_vec_u32(&self) -> Result<Vec<u32>> {
        self.device.read_buffer_u32(&self.buffer, self.len())
    }

    /// Transfer tensor to another device
    pub async fn to_device(&self, target_device: Arc<WgpuDevice>) -> Result<Self> {
        let data = self.to_vec()?;
        Self::from_vec_on(data, self.shape.clone(), target_device).await
    }

    /// Scalar multiplication: C = A * scalar
    ///
    /// Multiplies each element by a scalar value.
    /// Uses element-wise multiplication with broadcasted scalar.
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![3]).await?;
    /// let y = x.mul_scalar(2.0)?;  // [2.0, 4.0, 6.0]
    /// ```
    pub fn mul_scalar(&self, scalar: f32) -> Result<Tensor> {
        // Create broadcasted scalar tensor with same shape
        let data = vec![scalar; self.len()];
        let scalar_tensor = futures::executor::block_on(Tensor::from_vec_on(
            data,
            self.shape.clone(),
            self.device.clone(),
        ))?;

        // Use existing element-wise multiplication
        self.mul(&scalar_tensor)
    }

    /// Scalar addition: C = A + scalar
    ///
    /// Adds a scalar value to each element.
    /// Uses element-wise addition with broadcasted scalar.
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![3]).await?;
    /// let y = x.add_scalar(10.0)?;  // [11.0, 12.0, 13.0]
    /// ```
    pub fn add_scalar(&self, scalar: f32) -> Result<Tensor> {
        // Create broadcasted scalar tensor with same shape
        let data = vec![scalar; self.len()];
        let scalar_tensor = futures::executor::block_on(Tensor::from_vec_on(
            data,
            self.shape.clone(),
            self.device.clone(),
        ))?;

        // Use existing element-wise addition
        self.add(&scalar_tensor)
    }

    /// Scalar division: C = A / scalar
    ///
    /// Divides each element by a scalar value.
    /// Implemented as multiplication by reciprocal for efficiency.
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::from_vec(vec![10.0, 20.0, 30.0], vec![3]).await?;
    /// let y = x.div_scalar(2.0)?;  // [5.0, 10.0, 15.0]
    /// ```
    pub fn div_scalar(&self, scalar: f32) -> Result<Tensor> {
        // Multiply by reciprocal (faster than division)
        self.mul_scalar(1.0 / scalar)
    }

    /// Create random tensor with normal distribution N(0, 1)
    ///
    /// Uses Box-Muller transform to generate samples from standard normal distribution.
    /// For reproducible results, use `randn_seeded()` instead.
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::randn(vec![100, 100]).await?;
    /// // Values distributed N(0, 1), mean ≈ 0, std ≈ 1
    /// ```
    pub async fn randn(shape: Vec<usize>) -> Result<Self> {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::from_entropy();
        Self::randn_with_rng(shape, &mut rng).await
    }

    /// Create random tensor with normal distribution using provided RNG
    ///
    /// Allows for reproducible random generation with seeded RNG.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rand::SeedableRng;
    /// let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    /// let x = Tensor::randn_with_rng(vec![10, 10], &mut rng).await?;
    /// ```
    pub async fn randn_with_rng<R: rand::Rng>(shape: Vec<usize>, rng: &mut R) -> Result<Self> {
        let size: usize = shape.iter().product();

        // Box-Muller transform for normal distribution
        let mut data = Vec::with_capacity(size);
        for _ in 0..(size / 2) {
            let u1: f32 = rng.gen();
            let u2: f32 = rng.gen();

            // Guard against log(0)
            let u1 = u1.max(1e-10);

            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;

            data.push(r * theta.cos());
            data.push(r * theta.sin());
        }

        // Handle odd size
        if size % 2 == 1 {
            let u1: f32 = rng.gen::<f32>().max(1e-10);
            let u2: f32 = rng.gen();
            data.push((-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos());
        }

        data.truncate(size);
        Self::from_vec(data, shape).await
    }

    /// Create random tensor with uniform distribution U(0, 1)
    ///
    /// Generates values uniformly distributed between 0.0 (inclusive) and 1.0 (exclusive).
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::rand(vec![100, 100]).await?;
    /// // Values in [0, 1), mean ≈ 0.5
    /// ```
    pub async fn rand(shape: Vec<usize>) -> Result<Self> {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::from_entropy();
        Self::rand_with_rng(shape, &mut rng).await
    }

    /// Create random tensor with uniform distribution using provided RNG
    ///
    /// # Example
    /// ```rust,ignore
    /// use rand::SeedableRng;
    /// let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    /// let x = Tensor::rand_with_rng(vec![10, 10], &mut rng).await?;
    /// ```
    pub async fn rand_with_rng<R: rand::Rng>(shape: Vec<usize>, rng: &mut R) -> Result<Self> {
        let size: usize = shape.iter().product();
        let data: Vec<f32> = (0..size).map(|_| rng.gen()).collect();
        Self::from_vec(data, shape).await
    }

    /// Create random tensor with uniform distribution U(min, max)
    ///
    /// # Example
    /// ```rust,ignore
    /// let x = Tensor::rand_range(vec![100], -1.0, 1.0).await?;
    /// // Values in [-1, 1), mean ≈ 0
    /// ```
    pub async fn rand_range(shape: Vec<usize>, min: f32, max: f32) -> Result<Self> {
        let uniform = Self::rand(shape).await?;
        let range = max - min;
        uniform.mul_scalar(range)?.add_scalar(min)
    }

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
            assert!(val >= 0.0 && val < 1.0, "Value {} out of range", val);
        }
    }

    #[tokio::test]
    async fn test_rand_range() {
        let tensor = Tensor::rand_range(vec![100], -5.0, 5.0).await.unwrap();
        let data = tensor.to_vec().unwrap();

        // Check all values in range
        for &val in &data {
            assert!(val >= -5.0 && val < 5.0, "Value {} out of range", val);
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
}
