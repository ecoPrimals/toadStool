//! Reservoir state update operation for Echo State Networks
//!
//! Updates the reservoir (hidden) state based on input and recurrent connections.
//! This is the core dynamics of Echo State Networks, implementing the reservoir equation:
//!
//! **x(t+1) = (1-α)·x(t) + α·tanh(W_in·u(t) + W_res·x(t))**
//!
//! Where:
//! - x(t) is the reservoir state at time t
//! - u(t) is the input at time t
//! - W_in is the input weight matrix
//! - W_res is the recurrent (reservoir) weight matrix
//! - α is the leak rate (0 < α ≤ 1)
//!
//! # Echo State Property
//!
//! The Echo State Property ensures that the reservoir state asymptotically
//! depends only on the input history, not on the initial conditions. This is
//! achieved by:
//! - Spectral radius < 1.0 (typically 0.9-0.99)
//! - Leak rate for temporal integration
//! - Nonlinear activation (tanh)
//!
//! # Example
//!
//! ```no_run
//! use barracuda::reservoir_update;
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! let state = vec![0.0; 100];      // Current reservoir state
//! let input = vec![1.0, 0.5, -0.3]; // Input vector
//! let w_in = vec![/* input weights */];
//! let w_res = vec![/* reservoir weights */];
//!
//! // Update reservoir state
//! let new_state = reservoir_update(
//!     &device.device,
//!     &device.queue,
//!     &state,
//!     &input,
//!     &w_in,
//!     &w_res,
//!     0.3,  // leak_rate
//! ).await?;
//! # Ok(())
//! # }
//! ```

use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Update reservoir state for Echo State Networks
///
/// # Arguments
///
/// * `device` - The `wgpu` device
/// * `queue` - The `wgpu` queue
/// * `state` - Current reservoir state (N elements)
/// * `input` - Input vector (M elements)
/// * `w_in` - Input weights (N×M matrix, row-major)
/// * `w_res` - Reservoir weights (N×N matrix, row-major)
/// * `leak_rate` - Leak rate α ∈ (0, 1], controls temporal integration
///
/// # Returns
///
/// Updated reservoir state (N elements)
///
/// # Errors
///
/// Returns `BarracudaError` if:
/// - State, input, or weight dimensions are invalid
/// - Leak rate out of range
/// - GPU execution fails
pub async fn reservoir_update(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    state: &[f32],
    input: &[f32],
    w_in: &[f32],
    w_res: &[f32],
    leak_rate: f32,
) -> BarracudaResult<Vec<f32>> {
    let n = state.len() as u32;
    let m = input.len() as u32;

    if n == 0 || m == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "State and input must be non-empty".to_string(),
        });
    }

    if w_in.len() != (n * m) as usize {
        return Err(BarracudaError::InvalidInput {
            message: format!("w_in must be {}×{} (got {} elements)", n, m, w_in.len()),
        });
    }

    if w_res.len() != (n * n) as usize {
        return Err(BarracudaError::InvalidInput {
            message: format!("w_res must be {}×{} (got {} elements)", n, n, w_res.len()),
        });
    }

    if leak_rate <= 0.0 || leak_rate > 1.0 {
        return Err(BarracudaError::InvalidInput {
            message: "Leak rate must be in (0, 1]".to_string(),
        });
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Reservoir Update Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("reservoir_update.wgsl"))),
    });

    let state_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("State Input"),
        contents: bytemuck::cast_slice(state),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input"),
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let w_in_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input Weights"),
        contents: bytemuck::cast_slice(w_in),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let w_res_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Reservoir Weights"),
        contents: bytemuck::cast_slice(w_res),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output"),
        size: (n * std::mem::size_of::<f32>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        n: u32,
        m: u32,
        leak_rate: f32,
    }

    let params = Params { n, m, leak_rate };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Reservoir Update Layout"),
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
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
        label: Some("Reservoir Update Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: w_in_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: w_res_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Reservoir Update Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Reservoir Update Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "reservoir_update",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Reservoir Update Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Reservoir Update Pass"),
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
    async fn test_reservoir_update_basic() {
        let device = WgpuDevice::new().await.unwrap();
        let n = 10;
        let m = 3;
        let state = vec![0.0; n];
        let input = vec![1.0, 0.5, -0.5];
        let w_in = vec![0.1; n * m];
        let w_res = vec![0.05; n * n];

        let result = reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            0.5,
        )
        .await
        .unwrap();
        assert_eq!(result.len(), n);
        assert!(result.iter().all(|&x| x.is_finite()));
        // With zero initial state, result should be non-zero after update
        assert!(result.iter().any(|&x| x.abs() > 1e-6));
    }

    #[tokio::test]
    async fn test_reservoir_update_edge_cases() {
        let device = WgpuDevice::new().await.unwrap();
        // Test with different leak rates
        let state = vec![0.5; 5];
        let input = vec![1.0];
        let w_in = vec![0.1; 5];
        let w_res = vec![0.05; 25];

        let result1 = reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            0.1,
        )
        .await
        .unwrap();
        let result2 = reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            0.9,
        )
        .await
        .unwrap();

        // Higher leak rate should produce different (more responsive) states
        assert_ne!(result1, result2);
    }

    #[tokio::test]
    async fn test_reservoir_update_boundary() {
        let device = WgpuDevice::new().await.unwrap();
        let state = vec![0.0; 5];
        let input = vec![1.0];
        let w_in = vec![0.1; 5];
        let w_res = vec![0.05; 25];

        // Invalid leak rates
        assert!(reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            0.0
        )
        .await
        .is_err());
        assert!(reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            1.5
        )
        .await
        .is_err());

        // Invalid dimensions
        let bad_w_in = vec![0.1; 3];
        assert!(reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &bad_w_in,
            &w_res,
            0.5
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn test_reservoir_update_large_tensor() {
        let device = WgpuDevice::new().await.unwrap();
        let n = 100;
        let m = 10;
        let state = vec![0.1; n];
        let input = vec![0.5; m];
        let w_in = vec![0.01; n * m];
        let w_res = vec![0.005; n * n];

        let result = reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            0.3,
        )
        .await
        .unwrap();
        assert_eq!(result.len(), n);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_reservoir_update_precision() {
        let device = WgpuDevice::new().await.unwrap();
        let state = vec![0.1, 0.2, 0.3];
        let input = vec![1.0];
        let w_in = vec![0.5, -0.3, 0.2];
        let w_res = vec![0.1, 0.0, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.1];

        let result = reservoir_update(
            &device.device,
            &device.queue,
            &state,
            &input,
            &w_in,
            &w_res,
            1.0,
        )
        .await
        .unwrap();

        // With leak_rate=1.0, new state = tanh(W_in·u + W_res·x)
        // Verify tanh bounds [-1, 1]
        assert!(result.iter().all(|&x| x >= -1.0 && x <= 1.0));
    }
}
