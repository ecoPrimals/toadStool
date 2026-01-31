//! Spectral radius computation using power iteration method
//!
//! Computes the spectral radius (largest absolute eigenvalue) of a matrix,
//! which is critical for ensuring the Echo State Property in reservoir computing.
//!
//! # Spectral Radius
//!
//! The spectral radius ρ(A) is the maximum absolute eigenvalue of matrix A:
//! **ρ(A) = max|λᵢ|**
//!
//! For Echo State Networks:
//! - ρ(W_res) < 1.0 ensures stability and the Echo State Property
//! - Typical values: 0.9-0.99 for good memory-forgetting tradeoff
//! - Higher values → longer memory, slower convergence
//! - Lower values → shorter memory, faster convergence
//!
//! # Power Iteration Algorithm
//!
//! 1. Start with random vector v
//! 2. Iterate: v ← A·v / ||A·v||
//! 3. Converges to dominant eigenvector
//! 4. Eigenvalue λ ≈ ||A·v|| / ||v||
//!
//! # Example
//!
//! ```no_run
//! use barracuda::spectral_radius;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! let matrix = vec![/* reservoir weights */];
//! let size = 100;
//!
//! // Compute spectral radius with 50 iterations
//! let rho = spectral_radius(
//!     &device.device,
//!     &device.queue,
//!     &matrix,
//!     size,
//!     50,
//! ).await?;
//!
//! println!("Spectral radius: {:.4}", rho);
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Compute spectral radius using power iteration
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue
/// * `matrix` - Square matrix (N×N, row-major)
/// * `size` - Matrix dimension N
/// * `iterations` - Number of power iterations (typically 50-100)
///
/// # Returns
///
/// Spectral radius (largest absolute eigenvalue)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Matrix dimensions invalid
/// - Iterations is zero
/// - GPU execution fails
pub async fn spectral_radius(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    matrix: &[f32],
    size: u32,
    iterations: u32,
) -> BarracudaResult<f32> {
    if size == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Size must be greater than zero".to_string(),
        });
    }

    if matrix.len() != (size * size) as usize {
        return Err(BarracudaError::InvalidInput {
            message: format!("Matrix must be {}×{} (got {} elements)", size, size, matrix.len()),
        });
    }

    if iterations == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Iterations must be greater than zero".to_string(),
        });
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Spectral Radius Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("spectral_radius.wgsl"))),
    });

    let matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Matrix"),
        contents: bytemuck::cast_slice(matrix),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Ping-pong buffers for power iteration
    let vector_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vector A"),
        contents: bytemuck::cast_slice(&vec![1.0 / (size as f32).sqrt(); size as usize]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let vector_b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vector B"),
        size: (size * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let norm_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Norm"),
        size: std::mem::size_of::<f32>() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        size: u32,
    }

    let params = Params { size };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Spectral Radius Layout"),
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Spectral Radius Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let matmul_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("MatMul Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "matrix_vector_multiply",
    });

    let normalize_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Normalize Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "normalize_vector",
    });

    // Power iteration loop
    for i in 0..iterations {
        let (src, dst) = if i % 2 == 0 {
            (&vector_a, &vector_b)
        } else {
            (&vector_b, &vector_a)
        };

        // Create bind group for this iteration
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Iteration Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: norm_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Power Iteration Encoder"),
        });

        // Step 1: Matrix-vector multiply
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MatMul Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&matmul_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((size + 255) / 256, 1, 1);
        }

        // Step 2: Normalize vector
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Normalize Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&normalize_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((size + 255) / 256, 1, 1);
        }

        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    // Read final norm (spectral radius)
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: std::mem::size_of::<f32>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });
    encoder.copy_buffer_to_buffer(&norm_buffer, 0, &staging_buffer, 0, std::mem::size_of::<f32>() as u64);
    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
    device.poll(wgpu::Maintain::Wait);
    receiver.await.map_err(|_| BarracudaError::ExecutionError { message: "Failed to receive buffer".to_string() })?
        .map_err(|e| BarracudaError::ExecutionError { message: format!("Buffer mapping failed: {:?}", e) })?;

    let data = buffer_slice.get_mapped_range();
    let result: f32 = bytemuck::cast_slice(&data)[0];
    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    #[tokio::test]
    async fn test_spectral_radius_basic() {
        let device = WgpuDevice::new().await.unwrap();
        // Identity matrix has spectral radius = 1.0
        let mut matrix = vec![0.0; 25];
        for i in 0..5 {
            matrix[i * 5 + i] = 1.0;
        }
        let result = spectral_radius(&device.device, &device.queue, &matrix, 5, 50).await.unwrap();
        assert!((result - 1.0).abs() < 0.1, "Identity matrix should have ρ≈1.0, got {}", result);
    }

    #[tokio::test]
    async fn test_spectral_radius_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        // Scaled identity: 0.5·I should have ρ = 0.5
        let mut matrix = vec![0.0; 25];
        for i in 0..5 {
            matrix[i * 5 + i] = 0.5;
        }
        let result = spectral_radius(&device.device, &device.queue, &matrix, 5, 50).await.unwrap();
        assert!((result - 0.5).abs() < 0.1, "0.5·I should have ρ≈0.5, got {}", result);
    }

    #[tokio::test]
    async fn test_spectral_radius_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let matrix = vec![1.0; 25];
        
        // Invalid inputs
        assert!(spectral_radius(&device.device, &device.queue, &matrix, 0, 50).await.is_err());
        assert!(spectral_radius(&device.device, &device.queue, &matrix, 5, 0).await.is_err());
        
        let bad_matrix = vec![1.0; 20];
        assert!(spectral_radius(&device.device, &device.queue, &bad_matrix, 5, 50).await.is_err());
    }

    #[tokio::test]
    async fn test_spectral_radius_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        // Large diagonal matrix
        let mut matrix = vec![0.0; 10000];
        for i in 0..100 {
            matrix[i * 100 + i] = 0.8;
        }
        let result = spectral_radius(&device.device, &device.queue, &matrix, 100, 50).await.unwrap();
        assert!((result - 0.8).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_spectral_radius_precision() {
        let device = WgpuDevice::new().await.unwrap();
        // Test convergence: more iterations should give more precise result
        let mut matrix = vec![0.0; 25];
        for i in 0..5 {
            matrix[i * 5 + i] = 0.9;
        }
        
        let result1 = spectral_radius(&device.device, &device.queue, &matrix, 5, 10).await.unwrap();
        let result2 = spectral_radius(&device.device, &device.queue, &matrix, 5, 100).await.unwrap();
        
        // Both should be close to 0.9, but more iterations = better precision
        assert!((result1 - 0.9).abs() < 0.2);
        assert!((result2 - 0.9).abs() < 0.1);
    }
}
