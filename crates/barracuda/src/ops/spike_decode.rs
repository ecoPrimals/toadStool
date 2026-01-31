//! Spike decoding operation
//!
//! Converts spike trains back into continuous-valued signals for neuromorphic computing.
//! Uses rate decoding: higher spike counts → higher output values.
//!
//! # Neuromorphic Computing
//!
//! Spike decoding is the inverse of spike encoding, allowing us to read outputs
//! from spiking neural networks (SNNs) and convert them back to continuous values.
//!
//! # Decoding Strategy
//!
//! **Rate Decoding**: Spike frequency maps to output value
//! - 0 spikes → 0.0
//! - Maximum spikes → 1.0
//! - Linear mapping between extremes
//!
//! # Example
//!
//! ```no_run
//! use barracuda::spike_decode;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Decode SNN output spikes back to continuous values
//! let spike_counts = vec![0, 25, 50, 75, 100];  // Spike counts from SNN
//! let time_steps = 100;  // Total time steps used
//!
//! let values = spike_decode(
//!     &device.device,
//!     &device.queue,
//!     &spike_counts,
//!     time_steps,
//! ).await?;
//!
//! // Output: continuous values (0.0, 0.25, 0.5, 0.75, 1.0)
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Decode spike trains into continuous values using rate decoding
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `spike_counts` - Spike counts per neuron (0 to `time_steps`)
/// * `time_steps` - Total time steps used for encoding
///
/// # Returns
///
/// Vector of continuous values (normalized 0.0-1.0)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Input is empty
/// - Time steps is zero
/// - GPU execution fails
///
/// # Example
///
/// ```no_run
/// # use barracuda::spike_decode;
/// # use barracuda::WgpuDevice;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let device = WgpuDevice::new().await?;
/// let spikes = vec![0, 50, 100];
/// let values = spike_decode(&device.device, &device.queue, &spikes, 100).await?;
/// // Expected: [0.0, 0.5, 1.0]
/// # Ok(())
/// # }
/// ```
pub async fn spike_decode(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    spike_counts: &[u32],
    time_steps: u32,
) -> BarracudaResult<Vec<f32>> {
    if spike_counts.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Spike counts cannot be empty".to_string(),
        });
    }

    if time_steps == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Time steps must be greater than zero".to_string(),
        });
    }

    let n = spike_counts.len() as u32;

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Spike Decode Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("spike_decode.wgsl"))),
    });

    // Create input buffer
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Spike Decode Input Buffer"),
        contents: bytemuck::cast_slice(spike_counts),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Create output buffer
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spike Decode Output Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create params buffer
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        time_steps: u32,
        _padding: [u32; 2],
    }

    let params = Params {
        n,
        time_steps,
        _padding: [0; 2],
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Spike Decode Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Spike Decode Bind Group Layout"),
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

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Spike Decode Bind Group"),
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

    // Create pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Spike Decode Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Spike Decode Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "spike_decode",
    });

    // Execute
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Spike Decode Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Spike Decode Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }

    // Read back output
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spike Decode Staging Buffer"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (n * std::mem::size_of::<f32>() as u32) as u64,
    );

    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .await
        .map_err(|_| BarracudaError::ExecutionError {
            message: "Failed to receive buffer mapping result".to_string(),
        })?
        .map_err(|e| BarracudaError::ExecutionError {
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
    async fn test_spike_decode_basic() {
        let device = WgpuDevice::new().await.unwrap();

        // Test basic rate decoding
        let spike_counts = vec![0, 25, 50, 75, 100];
        let time_steps = 100;

        let result = spike_decode(&device.device, &device.queue, &spike_counts, time_steps)
            .await
            .unwrap();

        assert_eq!(result.len(), spike_counts.len());

        // Check approximate mapping (allow small tolerance)
        assert!((result[0] - 0.0).abs() < 0.01);
        assert!((result[1] - 0.25).abs() < 0.01);
        assert!((result[2] - 0.5).abs() < 0.01);
        assert!((result[3] - 0.75).abs() < 0.01);
        assert!((result[4] - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_spike_decode_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();

        // All zeros
        let zeros = vec![0; 10];
        let result = spike_decode(&device.device, &device.queue, &zeros, 50)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| x < 0.01));

        // All maximum
        let maxes = vec![50; 10];
        let result = spike_decode(&device.device, &device.queue, &maxes, 50)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 0.01));

        // Single element
        let single = vec![50];
        let result = spike_decode(&device.device, &device.queue, &single, 100)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_spike_decode_boundary() {
        let device = WgpuDevice::new().await.unwrap();

        // Very small time steps
        let spikes = vec![1];
        let result = spike_decode(&device.device, &device.queue, &spikes, 1)
            .await
            .unwrap();
        assert!((result[0] - 1.0).abs() < 0.01);

        // Large time steps
        let spikes = vec![500];
        let result = spike_decode(&device.device, &device.queue, &spikes, 1000)
            .await
            .unwrap();
        assert!((result[0] - 0.5).abs() < 0.01);

        // Empty input should error
        let empty: Vec<u32> = vec![];
        let result = spike_decode(&device.device, &device.queue, &empty, 100).await;
        assert!(result.is_err());

        // Zero time steps should error
        let result = spike_decode(&device.device, &device.queue, &spikes, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spike_decode_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();

        // Large input (simulating SNN output layer)
        let large_spikes: Vec<u32> = (0..10000).map(|i| (i % 100) as u32).collect();
        let result = spike_decode(&device.device, &device.queue, &large_spikes, 100)
            .await
            .unwrap();

        assert_eq!(result.len(), large_spikes.len());

        // Verify some samples
        for i in 0..100 {
            let expected = i as f32 / 100.0;
            assert!((result[i] - expected).abs() < 0.01);
        }
    }

    #[tokio::test]
    async fn test_spike_decode_precision() {
        let device = WgpuDevice::new().await.unwrap();

        // Test precision with various spike counts
        let spike_counts = vec![100, 333, 667, 900];
        let time_steps = 1000;

        let result = spike_decode(&device.device, &device.queue, &spike_counts, time_steps)
            .await
            .unwrap();

        // Check precision (allow ±1% tolerance)
        assert!((result[0] - 0.1).abs() < 0.01);
        assert!((result[1] - 0.333).abs() < 0.01);
        assert!((result[2] - 0.667).abs() < 0.01);
        assert!((result[3] - 0.9).abs() < 0.01);

        // All results should be within valid range [0.0, 1.0]
        assert!(result.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }
}
