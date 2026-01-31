//! Reservoir initialization operation for Echo State Networks
//!
//! Generates random reservoir (recurrent) weight matrices with controlled
//! spectral radius for Echo State Networks, a type of Reservoir Computing.
//!
//! # Reservoir Computing
//!
//! Reservoir Computing is a framework for training Recurrent Neural Networks (RNNs)
//! where the recurrent weights are fixed and randomly initialized, and only the
//! output (readout) layer is trained. Key properties:
//! - Fixed random recurrent weights
//! - Controlled spectral radius for stability
//! - Sparse connectivity for efficiency
//! - Echo State Property for memory
//!
//! # Algorithm
//!
//! **Random matrix with spectral control**:
//! 1. Generate random matrix from uniform distribution
//! 2. Apply sparsity mask (fraction of zeros)
//! 3. Scale to target spectral radius
//! 4. Return normalized reservoir matrix
//!
//! # Example
//!
//! ```no_run
//! use barracuda::reservoir_init;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Initialize 100-neuron reservoir with 90% sparsity, spectral radius 0.9
//! let reservoir = reservoir_init(
//!     &device.device,
//!     &device.queue,
//!     100,      // reservoir size
//!     0.9,      // spectral radius
//!     0.1,      // connectivity (10% non-zero)
//!     12345     // random seed
//! ).await?;
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Initialize reservoir weight matrix for Echo State Networks
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue  
/// * `size` - Reservoir size (N×N matrix)
/// * `spectral_radius` - Target spectral radius (typically 0.9-0.99)
/// * `connectivity` - Fraction of non-zero weights (0.0-1.0)
/// * `seed` - Random seed for reproducibility
///
/// # Returns
///
/// Flattened N×N reservoir matrix (row-major order)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Size is zero
/// - Spectral radius or connectivity out of range
/// - GPU execution fails
pub async fn reservoir_init(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
    spectral_radius: f32,
    connectivity: f32,
    seed: u32,
) -> BarracudaResult<Vec<f32>> {
    if size == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Size must be greater than zero".to_string(),
        });
    }

    if spectral_radius <= 0.0 || spectral_radius > 2.0 {
        return Err(BarracudaError::InvalidInput {
            message: "Spectral radius must be in (0, 2]".to_string(),
        });
    }

    if connectivity <= 0.0 || connectivity > 1.0 {
        return Err(BarracudaError::InvalidInput {
            message: "Connectivity must be in (0, 1]".to_string(),
        });
    }

    let n = size * size;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Reservoir Init Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("reservoir_init.wgsl"))),
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Reservoir Output"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        size: u32,
        n: u32,
        spectral_radius: f32,
        connectivity: f32,
        seed: u32,
    }

    let params = Params {
        size,
        n,
        spectral_radius,
        connectivity,
        seed,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Reservoir Init Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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
        label: Some("Reservoir Init Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Reservoir Init Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Reservoir Init Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "reservoir_init",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Reservoir Init Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Reservoir Init Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 255) / 256, 1, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (n * std::mem::size_of::<f32>() as u32) as u64);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
    device.poll(wgpu::Maintain::Wait);
    receiver.await.map_err(|_| BarracudaError::ExecutionError { message: "Failed to receive buffer".to_string() })?
        .map_err(|e| BarracudaError::ExecutionError { message: format!("Buffer mapping failed: {:?}", e) })?;

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
    async fn test_reservoir_init_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let result = reservoir_init(&device.device, &device.queue, 10, 0.9, 0.1, 42).await.unwrap();
        assert_eq!(result.len(), 100);
        assert!(result.iter().all(|&x| x.is_finite()));
        // Check sparsity (approximately 90% zeros)
        let zero_count = result.iter().filter(|&&x| x.abs() < 1e-6).count();
        assert!(zero_count > 80 && zero_count < 95, "Expected ~90 zeros, got {}", zero_count);
    }

    #[tokio::test]
    async fn test_reservoir_init_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        // Fully connected (connectivity = 1.0)
        let result = reservoir_init(&device.device, &device.queue, 5, 0.5, 1.0, 42).await.unwrap();
        let zero_count = result.iter().filter(|&&x| x.abs() < 1e-6).count();
        assert!(zero_count < 5, "With full connectivity, should have few zeros");

        // Different spectral radius
        let result2 = reservoir_init(&device.device, &device.queue, 5, 0.1, 0.5, 42).await.unwrap();
        assert!(result2.iter().all(|&x| x.abs() < 1.0));
    }

    #[tokio::test]
    async fn test_reservoir_init_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        assert!(reservoir_init(&device.device, &device.queue, 0, 0.9, 0.1, 42).await.is_err());
        assert!(reservoir_init(&device.device, &device.queue, 10, 0.0, 0.1, 42).await.is_err());
        assert!(reservoir_init(&device.device, &device.queue, 10, 0.9, 0.0, 42).await.is_err());
        assert!(reservoir_init(&device.device, &device.queue, 10, 0.9, 1.5, 42).await.is_err());
    }

    #[tokio::test]
    async fn test_reservoir_init_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let result = reservoir_init(&device.device, &device.queue, 100, 0.9, 0.1, 42).await.unwrap();
        assert_eq!(result.len(), 10000);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_reservoir_init_precision() {
        let device = WgpuDevice::new().await.unwrap();
        // Test reproducibility with same seed
        let result1 = reservoir_init(&device.device, &device.queue, 10, 0.9, 0.1, 12345).await.unwrap();
        let result2 = reservoir_init(&device.device, &device.queue, 10, 0.9, 0.1, 12345).await.unwrap();
        assert_eq!(result1.len(), result2.len());
        for (a, b) in result1.iter().zip(result2.iter()) {
            assert!((a - b).abs() < 1e-5, "Same seed should produce same results");
        }
    }
}
