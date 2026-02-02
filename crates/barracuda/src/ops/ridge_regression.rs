//! Ridge regression for Echo State Network readout layer training
//!
//! Trains the output layer of an ESN using ridge regression (L2-regularized
//! least squares). This is the supervised learning phase where only the
//! readout weights are trained, while reservoir weights remain fixed.
//!
//! # Ridge Regression
//!
//! Solves: **W_out = (X^T·X + λI)^(-1)·X^T·Y**
//!
//! Where:
//! - X is the reservoir state matrix (T×N, T time steps, N neurons)
//! - Y is the target output matrix (T×M, M output dimensions)
//! - λ is the regularization parameter (prevents overfitting)
//! - W_out is the readout weight matrix (N×M)
//!
//! # Regularization
//!
//! Ridge regression adds L2 penalty to prevent overfitting:
//! - λ = 0: No regularization (may overfit)
//! - λ small (1e-8): Minimal regularization
//! - λ large (1.0): Strong regularization (may underfit)
//! - Typical: 1e-6 to 1e-3
//!
//! # Example
//!
//! ```no_run
//! use barracuda::ridge_regression;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Reservoir states collected over time
//! let states = vec![/* T×N matrix */];
//! // Target outputs
//! let targets = vec![/* T×M matrix */];
//!
//! // Train readout layer
//! let w_out = ridge_regression(
//!     &device.device,
//!     &device.queue,
//!     &states,
//!     &targets,
//!     100,    // N neurons
//!     10,     // T time steps
//!     1,      // M outputs
//!     1e-6,   // regularization
//! ).await?;
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Train ESN readout layer using ridge regression
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue
/// * `states` - Reservoir states (T×N matrix, row-major)
/// * `targets` - Target outputs (T×M matrix, row-major)
/// * `n` - Number of reservoir neurons (N)
/// * `t` - Number of time steps (T)
/// * `m` - Number of output dimensions (M)
/// * `regularization` - Ridge parameter λ > 0
///
/// # Returns
///
/// Readout weights W_out (N×M matrix, row-major)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - Invalid dimensions
/// - Regularization ≤ 0
/// - GPU execution fails
pub async fn ridge_regression(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    states: &[f32],
    targets: &[f32],
    n: u32,
    t: u32,
    m: u32,
    regularization: f32,
) -> BarracudaResult<Vec<f32>> {
    if n == 0 || t == 0 || m == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Dimensions must be greater than zero".to_string(),
        });
    }

    if states.len() != (t * n) as usize {
        return Err(BarracudaError::InvalidInput {
            message: format!("States must be {}×{} (got {} elements)", t, n, states.len()),
        });
    }

    if targets.len() != (t * m) as usize {
        return Err(BarracudaError::InvalidInput {
            message: format!(
                "Targets must be {}×{} (got {} elements)",
                t,
                m,
                targets.len()
            ),
        });
    }

    if regularization <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: "Regularization must be positive".to_string(),
        });
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Ridge Regression Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("ridge_regression.wgsl"))),
    });

    let states_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("States"),
        contents: bytemuck::cast_slice(states),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let targets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Targets"),
        contents: bytemuck::cast_slice(targets),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output"),
        size: (n * m * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        t: u32,
        m: u32,
        regularization: f32,
    }

    let params = Params {
        n,
        t,
        m,
        regularization,
    };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Ridge Regression Layout"),
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
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Ridge Regression Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: states_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: targets_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Ridge Regression Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Ridge Regression Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "ridge_regression",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Ridge Regression Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Ridge Regression Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups((n + 15) / 16, (m + 15) / 16, 1);
    }

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: (n * m * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &staging_buffer,
        0,
        (n * m * std::mem::size_of::<f32>() as u32) as u64,
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
            message: "Failed to receive buffer".to_string(),
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
    async fn test_ridge_regression_basic() {
        let device = WgpuDevice::new().await.unwrap();
        // Simple linear relationship: y = 2*x
        let states = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5×1 (t=5, n=1)
        let targets = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // 5×1 (t=5, m=1)

        let result = ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            5,
            1,
            1e-6,
        )
        .await
        .unwrap();
        assert_eq!(result.len(), 1);
        // Should learn weight ≈ 2.0
        assert!(
            (result[0] - 2.0).abs() < 0.5,
            "Expected weight ≈2.0, got {}",
            result[0]
        );
    }

    #[tokio::test]
    async fn test_ridge_regression_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        // Multiple outputs
        let states = vec![1.0, 2.0, 3.0]; // 3×1
        let targets = vec![1.0, 0.5, 2.0, 1.0, 3.0, 1.5]; // 3×2

        let result = ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            3,
            2,
            1e-6,
        )
        .await
        .unwrap();
        assert_eq!(result.len(), 2); // 1×2 weights
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_ridge_regression_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let states = vec![1.0; 10];
        let targets = vec![1.0; 10];

        // Invalid dimensions
        assert!(ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            0,
            10,
            1,
            1e-6
        )
        .await
        .is_err());
        assert!(ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            0,
            1,
            1e-6
        )
        .await
        .is_err());

        // Invalid regularization
        assert!(ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            10,
            1,
            0.0
        )
        .await
        .is_err());
        assert!(ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            10,
            1,
            -0.1
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn test_ridge_regression_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let n = 50;
        let t = 100;
        let m = 5;
        let states = vec![0.1; (t * n) as usize];
        let targets = vec![0.5; (t * m) as usize];

        let result = ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            n,
            t,
            m,
            1e-6,
        )
        .await
        .unwrap();
        assert_eq!(result.len(), (n * m) as usize);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_ridge_regression_precision() {
        let device = WgpuDevice::new().await.unwrap();
        // Perfect linear fit with noise
        let states = vec![1.0, 2.0, 3.0, 4.0];
        let targets = vec![3.0, 5.0, 7.0, 9.0]; // y = 2x + 1

        let result = ridge_regression(
            &device.device,
            &device.queue,
            &states,
            &targets,
            1,
            4,
            1,
            1e-8,
        )
        .await
        .unwrap();

        // With very low regularization, should fit closely
        // Note: Without bias term, best fit will be y ≈ 2.3x
        assert!(
            result[0] > 1.5 && result[0] < 3.0,
            "Weight should be reasonable, got {}",
            result[0]
        );
    }
}
