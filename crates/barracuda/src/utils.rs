//! Utility functions for barracuda operations
//!
//! Deep Debt: Common utilities extracted to avoid duplication

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// Read buffer data from GPU back to CPU
///
/// This is used by WGSL operations to retrieve computation results
pub fn read_buffer(
    device: &Arc<WgpuDevice>,
    buffer: &wgpu::Buffer,
    size: usize,
) -> Result<Vec<f32>> {
    let byte_size = (size * std::mem::size_of::<f32>()) as u64;

    // Create staging buffer for reading
    let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Read Buffer Staging"),
        size: byte_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Copy from GPU buffer to staging buffer
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Read Buffer Encoder"),
        });

    encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, byte_size);

    device.queue.submit(Some(encoder.finish()));

    // Map the staging buffer and read data
    let buffer_slice = staging_buffer.slice(..);

    // Create a channel for async notification
    let (sender, receiver) = std::sync::mpsc::channel();

    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).ok();
    });

    // Poll the device until mapping is complete
    device.device.poll(wgpu::Maintain::Wait);

    // Wait for the mapping to complete
    receiver
        .recv()
        .map_err(|e| crate::error::BarracudaError::device(format!("Buffer mapping failed: {}", e)))?
        .map_err(|e| crate::error::BarracudaError::device(format!("Buffer map error: {:?}", e)))?;

    // Read the data
    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

    // Clean up
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

/// Read u32 buffer data from GPU back to CPU
///
/// This is used by operations that return integer indices (argmax, argmin, etc.)
pub fn read_buffer_u32(
    device: &Arc<WgpuDevice>,
    buffer: &wgpu::Buffer,
    size: usize,
) -> Result<Vec<u32>> {
    let byte_size = (size * std::mem::size_of::<u32>()) as u64;

    // Create staging buffer for reading
    let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Read Buffer U32 Staging"),
        size: byte_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Copy from GPU buffer to staging buffer
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Read Buffer U32 Encoder"),
        });

    encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, byte_size);

    device.queue.submit(Some(encoder.finish()));

    // Map the staging buffer and read data
    let buffer_slice = staging_buffer.slice(..);

    // Create a channel for async notification
    let (sender, receiver) = std::sync::mpsc::channel();

    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).ok();
    });

    // Poll the device until mapping is complete
    device.device.poll(wgpu::Maintain::Wait);

    // Wait for the mapping to complete
    receiver
        .recv()
        .map_err(|e| crate::error::BarracudaError::device(format!("Buffer mapping failed: {}", e)))?
        .map_err(|e| crate::error::BarracudaError::device(format!("Buffer map error: {:?}", e)))?;

    // Read the data
    let data = buffer_slice.get_mapped_range();
    let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();

    // Clean up
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}
