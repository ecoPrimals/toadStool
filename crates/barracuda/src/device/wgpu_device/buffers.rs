//! Buffer allocation and data transfer — generic read_buffer<T: Pod> + typed helpers

use super::WgpuDevice;
use crate::error::{BarracudaError, Result};
use wgpu::util::DeviceExt;

impl WgpuDevice {
    /// Read typed values from a GPU buffer.
    ///
    /// Creates a staging buffer, copies data, maps it, and extracts via bytemuck.
    /// All typed read variants delegate here.
    pub(crate) fn read_buffer<T: bytemuck::Pod>(
        &self,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<T>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let byte_size = (count * std::mem::size_of::<T>()) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback staging"),
            size: byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, byte_size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("GPU buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();

        Ok(result)
    }

    /// Read f64 values from a GPU buffer.
    ///
    /// This is the canonical readback path — all GPU ops should use this.
    pub fn read_f64_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<f64>> {
        self.read_buffer::<f64>(buffer, count)
    }

    /// Read buffer to host memory (f32)
    pub fn read_buffer_f32(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<f32>> {
        self.read_buffer::<f32>(buffer, size)
    }

    /// Write data to buffer (f32)
    pub fn write_buffer_f32(&self, buffer: &wgpu::Buffer, data: &[f32]) -> Result<()> {
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(data));
        Ok(())
    }

    /// Read f64 buffer to host memory
    pub fn read_buffer_f64(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<f64>> {
        self.read_buffer::<f64>(buffer, size)
    }

    /// Read u32 buffer to host memory
    pub fn read_buffer_u32(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<u32>> {
        self.read_buffer::<u32>(buffer, size)
    }

    /// Create storage buffer (convenience helper)
    pub fn create_storage_buffer(&self, label: &str, data: &[u8]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    }

    /// Create uniform buffer (convenience helper)
    pub fn create_uniform_buffer<T: bytemuck::Pod>(&self, label: &str, data: &T) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

    /// Allocate buffer for u32 data
    pub fn create_buffer_u32(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA U32 Buffer"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    /// Allocate zero-initialized buffer for u32 data
    pub fn create_buffer_u32_zeros(&self, size: usize) -> Result<wgpu::Buffer> {
        let zeros = vec![0u32; size];
        Ok(self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("barraCUDA U32 Zeros Buffer"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }))
    }

    /// Allocate buffer for f64 data
    pub fn create_buffer_f64(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA F64 Buffer"),
            size: (size * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }
}
