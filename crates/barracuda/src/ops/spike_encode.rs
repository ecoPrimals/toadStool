//! Spike encoding operation
//!
//! Converts continuous-valued signals into spike trains for neuromorphic computing.
//! Uses rate coding: higher input values → higher spike rates.
//!
//! # Neuromorphic Computing
//!
//! Spike encoding is fundamental for interfacing with spiking neural networks (SNNs)
//! on neuromorphic hardware like BrainChip Akida, Intel Loihi, or IBM TrueNorth.
//!
//! # Encoding Strategy
//!
//! **Rate Coding**: Input value maps to spike frequency
//! - Value 0.0 → 0 spikes
//! - Value 1.0 → Maximum spikes
//! - Linear mapping between extremes
//!
//! # Example
//!
//! ```no_run
//! use barracuda::spike_encode;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Encode sensor values to spike trains
//! let sensor_values = vec![0.2, 0.5, 0.8, 1.0];  // Normalized inputs
//! let time_steps = 100;  // Encode over 100 time steps
//!
//! let spikes = spike_encode(
//!     &device.device,
//!     &device.queue,
//!     &sensor_values,
//!     time_steps,
//! ).await?;
//!
//! // Output: spike counts per input (0.2→20, 0.5→50, 0.8→80, 1.0→100)
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Encode continuous values into spike trains using rate coding
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `input` - Input values (normalized 0.0-1.0)
/// * `time_steps` - Number of time steps for encoding
///
/// # Returns
///
/// Vector of spike counts per input (0 to `time_steps`)
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
/// # use barracuda::spike_encode;
/// # use barracuda::WgpuDevice;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let device = WgpuDevice::new().await?;
/// let input = vec![0.0, 0.25, 0.5, 0.75, 1.0];
/// let spikes = spike_encode(&device.device, &device.queue, &input, 100).await?;
/// // Expected: [0, 25, 50, 75, 100]
/// # Ok(())
/// # }
/// ```
pub async fn spike_encode(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &[f32],
    time_steps: u32,
) -> BarracudaResult<Vec<u32>> {
    if input.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "Input cannot be empty".to_string(),
        });
    }

    if time_steps == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Time steps must be greater than zero".to_string(),
        });
    }

    let n = input.len() as u32;

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Spike Encode Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("spike_encode.wgsl"))),
    });

    // Create input buffer
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Spike Encode Input Buffer"),
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Create output buffer
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spike Encode Output Buffer"),
        size: (n * std::mem::size_of::<u32>() as u32) as u64,
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
        label: Some("Spike Encode Params Buffer"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Spike Encode Bind Group Layout"),
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
        label: Some("Spike Encode Bind Group"),
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
        label: Some("Spike Encode Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Spike Encode Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "spike_encode",
    });

    // Execute
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Spike Encode Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Spike Encode Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }

    // Read back output
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Spike Encode Staging Buffer"),
        size: (n * std::mem::size_of::<u32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (n * std::mem::size_of::<u32>() as u32) as u64,
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
    let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_spike_encode_basic() {
        let device = WgpuDevice::new().await.unwrap();

        // Test basic rate coding
        let input = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let time_steps = 100;

        let result = spike_encode(&device.device, &device.queue, &input, time_steps)
            .await
            .unwrap();

        assert_eq!(result.len(), input.len());

        // Check approximate mapping (allow ±1 for rounding)
        assert!(result[0] <= 1); // 0.0 → ~0
        assert!((result[1] as i32 - 25).abs() <= 1); // 0.25 → ~25
        assert!((result[2] as i32 - 50).abs() <= 1); // 0.5 → ~50
        assert!((result[3] as i32 - 75).abs() <= 1); // 0.75 → ~75
        assert!((result[4] as i32 - 100).abs() <= 1); // 1.0 → ~100
    }

    #[tokio::test]
    async fn test_spike_encode_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();

        // All zeros
        let zeros = vec![0.0; 10];
        let result = spike_encode(&device.device, &device.queue, &zeros, 50)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| x <= 1));

        // All ones
        let ones = vec![1.0; 10];
        let result = spike_encode(&device.device, &device.queue, &ones, 50)
            .await
            .unwrap();
        assert!(result.iter().all(|&x| (x as i32 - 50).abs() <= 1));

        // Single element
        let single = vec![0.5];
        let result = spike_encode(&device.device, &device.queue, &single, 100)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] as i32 - 50).abs() <= 1);
    }

    #[tokio::test]
    async fn test_spike_encode_boundary() {
        let device = WgpuDevice::new().await.unwrap();

        // Very small time steps
        let input = vec![0.5];
        let result = spike_encode(&device.device, &device.queue, &input, 1)
            .await
            .unwrap();
        assert!(result[0] <= 1);

        // Large time steps
        let result = spike_encode(&device.device, &device.queue, &input, 1000)
            .await
            .unwrap();
        assert!((result[0] as i32 - 500).abs() <= 2);

        // Empty input should error
        let empty: Vec<f32> = vec![];
        let result = spike_encode(&device.device, &device.queue, &empty, 100).await;
        assert!(result.is_err());

        // Zero time steps should error
        let result = spike_encode(&device.device, &device.queue, &input, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spike_encode_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();

        // Large input (simulating sensor array)
        let large_input: Vec<f32> = (0..10000).map(|i| (i % 100) as f32 / 100.0).collect();
        let result = spike_encode(&device.device, &device.queue, &large_input, 100)
            .await
            .unwrap();

        assert_eq!(result.len(), large_input.len());

        // Verify some samples
        for i in 0..100 {
            let expected = i;
            assert!((result[i] as i32 - expected as i32).abs() <= 1);
        }
    }

    #[tokio::test]
    async fn test_spike_encode_precision() {
        let device = WgpuDevice::new().await.unwrap();

        // Test precision with fractional values
        let input = vec![0.1, 0.333, 0.667, 0.9];
        let time_steps = 1000;

        let result = spike_encode(&device.device, &device.queue, &input, time_steps)
            .await
            .unwrap();

        // Check precision (allow ±1% tolerance)
        assert!((result[0] as f32 - 100.0).abs() < 10.0); // 0.1 → ~100
        assert!((result[1] as f32 - 333.0).abs() < 10.0); // 0.333 → ~333
        assert!((result[2] as f32 - 667.0).abs() < 10.0); // 0.667 → ~667
        assert!((result[3] as f32 - 900.0).abs() < 10.0); // 0.9 → ~900

        // All results should be within valid range
        assert!(result.iter().all(|&x| x <= time_steps));
    }
}
