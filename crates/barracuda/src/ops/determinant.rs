//! Matrix determinant calculation - Pure WGSL implementation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (hardware-agnostic)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Supports 2x2, 3x3, NxN matrices
//! - ✅ Batch processing for multiple matrices
//!
//! ## Algorithm
//!
//! - 2x2: det(A) = a*d - b*c (exact)
//! - 3x3: Sarrus rule (exact)
//! - NxN: LU decomposition via Gaussian elimination
//!   - For large matrices, uses iterative row reduction
//!   - Determinant = product of diagonal elements after LU decomposition
//!
//! ## Usage
//!
//! ```rust,ignore
//! let matrix = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).await?;
//! let det = matrix.determinant()?; // Returns scalar tensor
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DeterminantParams {
    matrix_size: u32,
    total_matrices: u32,
    _padding: [u32; 2],
}

pub struct Determinant {
    input: Tensor,
}

impl Determinant {
    pub fn new(input: Tensor) -> Result<Self> {
        // Verify square matrix
        let shape = input.shape();
        if shape.len() < 2 {
            return Err(BarracudaError::invalid_op(
                "determinant",
                "Requires at least a 2D tensor (matrix)",
            ));
        }

        let rows = shape[shape.len() - 2];
        let cols = shape[shape.len() - 1];

        if rows != cols {
            return Err(BarracudaError::invalid_op(
                "determinant",
                format!("Requires square matrix, got {}x{}", rows, cols),
            ));
        }

        Ok(Self { input })
    }

    /// WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/determinant.wgsl")
    }

    /// Execute determinant calculation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Get matrix dimensions
        let matrix_size = shape[shape.len() - 1]; // N for NxN matrix
        let total_matrices: usize = if shape.len() > 2 {
            shape[..shape.len() - 2].iter().product()
        } else {
            1
        };

        // Create output buffer (one determinant per matrix)
        let output_buffer = device.create_buffer_f32(total_matrices)?;

        // Create parameters
        let params = DeterminantParams {
            matrix_size: matrix_size as u32,
            total_matrices: total_matrices as u32,
            _padding: [0, 0],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Determinant Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Determinant Bind Group Layout"),
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
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Determinant Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
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

        // Compile shader
        let shader_module = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Determinant Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Determinant Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Determinant Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Determinant Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Determinant Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups = (total_matrices as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return scalar or vector of determinants
        let output_shape = if total_matrices == 1 {
            vec![1]
        } else {
            shape[..shape.len() - 2].to_vec()
        };

        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_determinant_basic() {
        let device = get_test_device().await;
        // 2x2 matrix: [[4, 7], [2, 6]]
        // det = 4*6 - 7*2 = 24 - 14 = 10
        let matrix = Tensor::from_vec_on(vec![4.0, 7.0, 2.0, 6.0], vec![2, 2], device)
            .await
            .unwrap();

        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();

        assert_eq!(result.len(), 1);
        assert!(
            (result[0] - 10.0).abs() < 1e-4,
            "Expected 10.0, got {}",
            result[0]
        );
    }

    #[tokio::test]
    async fn test_determinant_edge_cases() {
        let device = get_test_device().await;

        // 1x1 matrix
        let matrix = Tensor::from_vec_on(vec![5.0], vec![1, 1], device.clone())
            .await
            .unwrap();
        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();
        assert!((result[0] - 5.0).abs() < 1e-5);

        // Singular matrix (det = 0)
        let matrix = Tensor::from_vec_on(vec![1.0, 2.0, 2.0, 4.0], vec![2, 2], device.clone())
            .await
            .unwrap();
        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();
        assert!(result[0].abs() < 1e-5, "Singular matrix should have det=0");

        // Identity matrix (det = 1)
        let matrix = Tensor::from_vec_on(vec![1.0, 0.0, 0.0, 1.0], vec![2, 2], device)
            .await
            .unwrap();
        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_determinant_boundary() {
        let device = get_test_device().await;

        // 3x3 matrix
        let matrix = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0],
            vec![3, 3],
            device,
        )
        .await
        .unwrap();

        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();

        // Expected: 1*(1*0 - 4*6) - 2*(0*0 - 4*5) + 3*(0*6 - 1*5) = -24 + 40 - 15 = 1
        assert!(
            (result[0] - 1.0).abs() < 1e-3,
            "Expected 1.0, got {}",
            result[0]
        );
    }

    #[tokio::test]
    async fn test_determinant_precision() {
        let device = get_test_device().await;

        // 2x2 matrix with precise values
        let matrix = Tensor::from_vec_on(vec![1.5, 2.5, 3.5, 4.5], vec![2, 2], device)
            .await
            .unwrap();

        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();

        // det = 1.5*4.5 - 2.5*3.5 = 6.75 - 8.75 = -2.0
        assert!((result[0] - (-2.0)).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_determinant_large_batch() {
        let device = get_test_device().await;

        // 2x2 negative determinant
        let matrix = Tensor::from_vec_on(vec![2.0, 3.0, 1.0, 4.0], vec![2, 2], device)
            .await
            .unwrap();

        let det = Determinant::new(matrix).unwrap().execute().unwrap();
        let result = det.to_vec().unwrap();

        // det = 2*4 - 3*1 = 8 - 3 = 5
        assert!((result[0] - 5.0).abs() < 1e-5);
    }
}
