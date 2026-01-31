//! Temporal pooling operation for spike trains
//!
//! Aggregates spike activity over time windows for neuromorphic computing.
//! Converts temporal spike patterns into rate-based representations.
//!
//! # Neuromorphic Computing
//!
//! Temporal pooling is essential for processing spike train outputs from SNNs.
//! It reduces temporal dimension while preserving information about firing rates.
//!
//! # Pooling Strategy
//!
//! **Window-based averaging**: Average spike rate over time windows
//! - Input: Spike flags (0 or 1) over time
//! - Window: Fixed size aggregation period
//! - Output: Average firing rate per window
//!
//! # Example
//!
//! ```no_run
//! use barracuda::temporal_pool;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Pool spike train over 10-step windows
//! let spikes = vec![1.0, 0.0, 1.0, 1.0, 0.0,  // Window 1: 3/5 = 0.6
//!                   0.0, 0.0, 0.0, 1.0, 1.0]; // Window 2: 2/5 = 0.4
//! let pooled = temporal_pool(&device.device, &device.queue, &spikes, 5).await?;
//! // Result: [0.6, 0.4]
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Pool spike trains over time windows
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `spikes` - Spike flags over time (0.0 or 1.0)
/// * `window_size` - Size of pooling window
///
/// # Returns
///
/// Vector of average firing rates per window
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Input is empty
/// - Window size is zero
/// - GPU execution fails
pub async fn temporal_pool(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    spikes: &[f32],
    window_size: u32,
) -> BarracudaResult<Vec<f32>> {
    if spikes.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Spikes cannot be empty".to_string(),
        });
    }

    if window_size == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Window size must be greater than zero".to_string(),
        });
    }

    let n = spikes.len() as u32;
    let num_windows = (n + window_size - 1) / window_size;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Temporal Pool Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("temporal_pool.wgsl"))),
    });

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Temporal Pool Input"),
        contents: bytemuck::cast_slice(spikes),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Temporal Pool Output"),
        size: (num_windows * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        window_size: u32,
        num_windows: u32,
        _padding: u32,
    }

    let params = Params {
        n,
        window_size,
        num_windows,
        _padding: 0,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Temporal Pool Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Temporal Pool Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Temporal Pool Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Temporal Pool Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Temporal Pool Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "temporal_pool",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Temporal Pool Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Temporal Pool Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((num_windows + 255) / 256, 1, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Temporal Pool Staging"),
        size: (num_windows * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (num_windows * std::mem::size_of::<f32>() as u32) as u64,
    );

    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver.await.map_err(|_| BarracudaError::ExecutionError {
        message: "Failed to receive buffer mapping result".to_string(),
    })?.map_err(|e| BarracudaError::ExecutionError {
        message: format!("Buffer mapping failed: {:?}", e),
    })?;

    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_temporal_pool_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let spikes = vec![1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0];
        let result = temporal_pool(&device.device, &device.queue, &spikes, 5).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.6).abs() < 0.01);
        assert!((result[1] - 0.4).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_temporal_pool_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        let zeros = vec![0.0; 20];
        let result = temporal_pool(&device.device, &device.queue, &zeros, 5).await.unwrap();
        assert!(result.iter().all(|&x| x < 0.01));

        let ones = vec![1.0; 20];
        let result = temporal_pool(&device.device, &device.queue, &ones, 5).await.unwrap();
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 0.01));
    }

    #[tokio::test]
    async fn test_temporal_pool_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let single = vec![1.0];
        let result = temporal_pool(&device.device, &device.queue, &single, 1).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] - 1.0).abs() < 0.01);

        let empty: Vec<f32> = vec![];
        assert!(temporal_pool(&device.device, &device.queue, &empty, 5).await.is_err());
        assert!(temporal_pool(&device.device, &device.queue, &single, 0).await.is_err());
    }

    #[tokio::test]
    async fn test_temporal_pool_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let large: Vec<f32> = (0..10000).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
        let result = temporal_pool(&device.device, &device.queue, &large, 100).await.unwrap();
        assert_eq!(result.len(), 100);
        assert!(result.iter().all(|&x| (x - 0.5).abs() < 0.01));
    }

    #[tokio::test]
    async fn test_temporal_pool_precision() {
        let device = WgpuDevice::new().await.unwrap();
        let pattern = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        let result = temporal_pool(&device.device, &device.queue, &pattern, 5).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.6).abs() < 0.01);
        assert!((result[1] - 0.4).abs() < 0.01);
        assert!(result.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }
}
