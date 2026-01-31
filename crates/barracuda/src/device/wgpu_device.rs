//! Pure WGSL device - hardware-agnostic compute via WebGPU
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no separate CPU code!)
//! - wgpu handles execution on ANY device (GPU/CPU/NPU/TPU)
//! - Single implementation per operation
//! - Let wgpu experts handle backend optimization

use crate::error::{BarracudaError, Result};
use std::sync::Arc;

/// WebGPU device - executes WGSL on any hardware
///
/// wgpu automatically selects best backend:
/// - Vulkan (NVIDIA, AMD, Intel GPUs)
/// - Metal (Apple GPUs)
/// - DX12 (Windows GPUs)
/// - Software rasterizer (CPU fallback)
/// - Custom (NPU/TPU if driver available)
#[derive(Debug, Clone)]
pub struct WgpuDevice {
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
    adapter_info: wgpu::AdapterInfo,
}

impl WgpuDevice {
    /// Create new WebGPU device with auto-discovery
    ///
    /// **Deep Debt**: Discovers any available GPU, no hardcoding
    pub async fn new() -> Result<Self> {
        Self::new_with_backend(wgpu::Backends::all()).await
    }

    /// Create with specific backend (for testing/multi-GPU)
    pub async fn new_with_backend(backends: wgpu::Backends) -> Result<Self> {
        Self::new_with_filter(backends, |_| true).await
    }

    /// Create with custom filter (for specific GPU selection)
    pub async fn new_with_filter<F>(backends: wgpu::Backends, filter: F) -> Result<Self>
    where
        F: Fn(&wgpu::AdapterInfo) -> bool,
    {
        // Create instance (pure Rust runtime discovery)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Enumerate adapters
        let adapters = instance.enumerate_adapters(backends);

        if adapters.is_empty() {
            return Err(BarracudaError::device("No GPU adapters found"));
        }

        // Find matching adapter
        let adapter = adapters
            .into_iter()
            .find(|adapter: &wgpu::Adapter| filter(&adapter.get_info()))
            .ok_or_else(|| BarracudaError::device("No GPU matching filter"))?;

        let adapter_info = adapter.get_info();

        // Request device (runtime capability negotiation)
        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("barraCUDA Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {}", e)))?;

        // Log what device we're using
        log::info!(
            "barraCUDA initialized: {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Get device name (e.g., "NVIDIA RTX 4090", "llvmpipe (CPU)")
    pub fn name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Get device type (DiscreteGpu, IntegratedGpu, Cpu, etc.)
    pub fn device_type(&self) -> wgpu::DeviceType {
        self.adapter_info.device_type
    }

    /// Check if running on CPU fallback
    pub fn is_cpu(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }

    /// Access underlying wgpu device
    ///
    /// **Deep Debt**: Enables external consumers to use barraCUDA infrastructure
    /// for custom operations (e.g., homomorphic computing, neuromorphic, etc.)
    ///
    /// # Safety
    /// External users must ensure proper synchronization with the queue.
    /// Use `queue()` for command submission.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Access command queue
    ///
    /// **Deep Debt**: Enables external consumers to submit custom compute passes
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Create storage buffer (convenience helper)
    ///
    /// **Deep Debt**: Reduces boilerplate for external barraCUDA users
    ///
    /// # Example
    /// ```rust,no_run
    /// # use barracuda::prelude::*;
    /// # async fn example() -> Result<()> {
    /// let device = WgpuDevice::new().await?;
    /// let buffer = device.create_storage_buffer("my_data", &[1u8, 2, 3, 4]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_storage_buffer(&self, label: &str, data: &[u8]) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Create uniform buffer (convenience helper)
    ///
    /// **Deep Debt**: Type-safe uniform buffer creation
    ///
    /// # Example
    /// ```rust,no_run
    /// # use barracuda::prelude::*;
    /// # use bytemuck::{Pod, Zeroable};
    /// # async fn example() -> Result<()> {
    /// #[repr(C)]
    /// #[derive(Copy, Clone, Pod, Zeroable)]
    /// struct Params {
    ///     width: u32,
    ///     height: u32,
    /// }
    ///
    /// let device = WgpuDevice::new().await?;
    /// let params = Params { width: 1920, height: 1080 };
    /// let buffer = device.create_uniform_buffer("params", &params);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_uniform_buffer<T: bytemuck::Pod>(
        &self,
        label: &str,
        data: &T,
    ) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::bytes_of(data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Allocate buffer for f32 data
    pub fn create_buffer_f32(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    /// Compile WGSL shader
    pub fn compile_shader(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    /// Execute WGSL compute shader
    pub fn execute_compute(
        &self,
        shader_source: &str,
        bind_groups: &[&wgpu::BindGroup],
        workgroups: (u32, u32, u32),
    ) -> Result<()> {
        // Compile WGSL
        let shader = self.compile_shader(shader_source, Some("barraCUDA Operation"));

        // Create pipeline
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("barraCUDA Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Encode and submit
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("barraCUDA Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("barraCUDA Compute"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            for (i, bind_group) in bind_groups.iter().enumerate() {
                pass.set_bind_group(i as u32, bind_group, &[]);
            }
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}

impl WgpuDevice {
    /// Read buffer to host memory
    pub fn read_buffer_f32(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<f32>> {
        // Create staging buffer
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy GPU -> staging
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);

        // Wait for mapping
        futures::executor::block_on(receiver)
            .map_err(|_| BarracudaError::gpu("Failed to map buffer"))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        // Copy data
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Write data to buffer
    pub fn write_buffer_f32(&self, buffer: &wgpu::Buffer, data: &[f32]) -> Result<()> {
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(data));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wgpu_device_creation() {
        // Should always succeed (wgpu has CPU fallback)
        let device = WgpuDevice::new().await.unwrap();
        println!("barraCUDA device: {}", device.name());
        println!("Device type: {:?}", device.device_type());
        
        if device.is_cpu() {
            println!("✓ Using CPU fallback (software rasterizer)");
        } else {
            println!("✓ Using GPU acceleration");
        }
    }

    #[tokio::test]
    async fn test_buffer_operations() {
        let device = WgpuDevice::new().await.unwrap();

        // Create buffer
        let buffer = device.create_buffer_f32(10).unwrap();

        // Write data
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        device.write_buffer_f32(&buffer, &data).unwrap();

        // Read back
        let read_data = device.read_buffer_f32(&buffer, 10).unwrap();
        assert_eq!(read_data, data);
    }
}
