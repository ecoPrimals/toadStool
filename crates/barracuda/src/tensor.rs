//! Unified Tensor abstraction - hardware-agnostic tensor compute
//!
//! **Deep Debt Excellence**:
//! - Single Tensor type works on any device
//! - Self-knowledge: Tensor knows its device
//! - Automatic operations dispatch based on device
//! - Zero duplication across backends

use crate::device::{Auto, WgpuDevice};
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
    /// GPU buffer (wgpu handles CPU/GPU/NPU/TPU automatically!)
    buffer: wgpu::Buffer,

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
            buffer,
            shape,
            device,
            name: None,
        }
    }

    /// Get reference to buffer (internal use)
    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Create tensor from data (for testing and initialization)
    pub fn from_data<T: bytemuck::Pod>(
        data: &[T],
        shape: Vec<usize>,
        device: Arc<WgpuDevice>,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;
        
        let buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tensor Data"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            buffer,
            shape,
            device,
            name: None,
        })
    }
}

// Implement Clone for Tensor
impl Clone for Tensor {
    fn clone(&self) -> Self {
        // Create a new buffer and copy data
        let size = self.len();
        let new_buffer = self.device.create_buffer_f32(size).unwrap();
        
        // Copy data from old buffer to new buffer
        let mut encoder = self.device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tensor Clone Encoder"),
        });
        
        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &new_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        
        self.device.queue.submit(Some(encoder.finish()));
        
        Self {
            buffer: new_buffer,
            shape: self.shape.clone(),
            device: self.device.clone(),
            name: self.name.clone(),
        }
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
            buffer,
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

    /// Transfer tensor to another device
    pub async fn to_device(&self, target_device: Arc<WgpuDevice>) -> Result<Self> {
        let data = self.to_vec()?;
        Self::from_vec_on(data, self.shape.clone(), target_device).await
    }

    /// Reshape tensor (zero-copy when possible)
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Self> {
        // Validate element count matches
        let old_size: usize = self.shape.iter().product();
        let new_size: usize = new_shape.iter().product();

        if old_size != new_size {
            return Err(BarracudaError::shape_mismatch(vec![new_size], vec![old_size]));
        }

        // For now, simple implementation: read and recreate
        // TODO: Zero-copy reshape when striding allows
        let data = self.to_vec()?;
        futures::executor::block_on(Self::from_vec_on(data, new_shape, self.device.clone()))
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
}
