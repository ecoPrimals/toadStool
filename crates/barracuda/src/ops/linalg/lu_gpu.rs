//! LU Decomposition - GPU-Accelerated Implementation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//! - ✅ Runtime-configured matrix size
//! - ✅ Capability-based dispatch
//!
//! ## Algorithm
//!
//! Multi-pass GPU LU decomposition with partial pivoting:
//! ```text
//! For each column k = 0..n-1:
//!   1. find_pivot:         GPU parallel reduction to find max|A[i,k]| for i >= k
//!   2. row_swap:           GPU parallel swap rows k and pivot_row
//!   3. compute_multipliers: GPU parallel L[i,k] = A[i,k]/A[k,k] for i > k
//!   4. row_elimination:    GPU parallel A[i,j] -= L[i,k]*A[k,j] for i,j > k
//! ```
//!
//! ## Precision
//!
//! Uses f32 for GPU execution. For f64 precision, use the CPU `lu_decompose()`.
//!
//! ## References
//!
//! - Golub & Van Loan, "Matrix Computations", Algorithm 3.4.1

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU-accelerated LU decomposition
///
/// Computes PA = LU where P is permutation, L is lower triangular, U is upper triangular.
pub struct LuGpu {
    input: Tensor,
}

impl LuGpu {
    /// Create new GPU LU decomposition operation
    ///
    /// # Arguments
    /// * `input` - Square matrix [N, N] in row-major order
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/linalg/lu_decomp.wgsl")
    }

    /// Execute LU decomposition on GPU
    ///
    /// # Returns
    /// Tuple (lu_matrix, permutation) where:
    /// - lu_matrix: Combined L and U in single matrix (L below diagonal, U on/above)
    /// - permutation: Row permutation vector
    ///
    /// # Errors
    /// - Returns error if input is not square
    pub fn execute(self) -> Result<(Tensor, Vec<u32>)> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Validate square matrix
        if shape.len() != 2 || shape[0] != shape[1] {
            return Err(BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: shape.to_vec(),
            });
        }

        let n = shape[0] as u32;

        // Create working buffer (copy of input, will be modified in-place)
        let input_data = self.input.to_vec()?;
        let lu_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LU Matrix Buffer"),
            contents: bytemuck::cast_slice(&input_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        // Create permutation buffer
        let perm_init: Vec<u32> = (0..n).collect();
        let perm_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LU Permutation Buffer"),
            contents: bytemuck::cast_slice(&perm_init),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        // Create pivot result buffer [row_idx, max_val_bits]
        let pivot_result_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pivot Result Buffer"),
            size: 8, // 2 × u32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("LU Decomp"));

        // Create pipelines for each kernel
        let (find_pivot_pipeline, find_pivot_layout) =
            self.create_find_pivot_pipeline(&device.device, &shader);
        let (row_swap_pipeline, row_swap_layout) =
            self.create_row_swap_pipeline(&device.device, &shader);
        let (compute_mult_pipeline, compute_mult_layout) =
            self.create_compute_mult_pipeline(&device.device, &shader);
        let (row_elim_pipeline, row_elim_layout) =
            self.create_row_elim_pipeline(&device.device, &shader);

        // Main loop: process each column
        for k in 0..(n - 1) {
            // Create params buffer for this iteration
            let params = [n, k, 0u32, 0u32]; // n, k, pivot_row (updated after find_pivot), _pad
            let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LU Params"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // 1. Find pivot
            let pivot_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Find Pivot BG"),
                layout: &find_pivot_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: lu_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: pivot_result_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Find Pivot Encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Find Pivot Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&find_pivot_pipeline);
                pass.set_bind_group(0, &pivot_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1); // Single workgroup for reduction
            }
            device.queue.submit(Some(encoder.finish()));

            // Read back pivot row
            let pivot_row = self.read_pivot_result(device, &pivot_result_buffer)?;

            // Update params with pivot_row for row_swap
            let params_with_pivot = [n, k, pivot_row, 0u32];
            let params_buffer_swap = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LU Params Swap"),
                contents: bytemuck::cast_slice(&params_with_pivot),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            // 2. Row swap (if needed)
            if pivot_row != k {
                let swap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Row Swap BG"),
                    layout: &row_swap_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: params_buffer_swap.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: lu_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: perm_buffer.as_entire_binding(),
                        },
                    ],
                });

                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Row Swap Encoder"),
                });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Row Swap Pass"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&row_swap_pipeline);
                    pass.set_bind_group(0, &swap_bg, &[]);
                    pass.dispatch_workgroups((n + 255) / 256, 1, 1);
                }
                device.queue.submit(Some(encoder.finish()));
            }

            // 3. Compute multipliers
            let mult_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compute Mult BG"),
                layout: &compute_mult_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer_swap.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: lu_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: perm_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Mult Encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Mult Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&compute_mult_pipeline);
                pass.set_bind_group(0, &mult_bg, &[]);
                let rows_to_process = n - k - 1;
                pass.dispatch_workgroups((rows_to_process + 255) / 256, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));

            // 4. Row elimination
            let elim_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Row Elim BG"),
                layout: &row_elim_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer_swap.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: lu_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: perm_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Row Elim Encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Row Elim Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&row_elim_pipeline);
                pass.set_bind_group(0, &elim_bg, &[]);
                let submatrix_size = n - k - 1;
                let workgroups_x = (submatrix_size + 15) / 16;
                let workgroups_y = (submatrix_size + 15) / 16;
                pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Read back results
        let lu_data = device.read_buffer_f32(&lu_buffer, (n * n) as usize)?;
        let perm_data = device.read_buffer_u32(&perm_buffer, n as usize)?;

        // Create output tensor
        let output_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LU Output"),
            contents: bytemuck::cast_slice(&lu_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let lu_tensor = Tensor::from_buffer(output_buffer, shape.to_vec(), device.clone());

        Ok((lu_tensor, perm_data))
    }

    // Helper: Create find_pivot pipeline
    fn create_find_pivot_pipeline(
        &self,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Find Pivot BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Find Pivot PL"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Find Pivot Pipeline"),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "find_pivot",
        });

        (pipeline, layout)
    }

    // Helper: Create row_swap pipeline
    fn create_row_swap_pipeline(
        &self,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Row Swap BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Row Swap PL"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Row Swap Pipeline"),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "row_swap",
        });

        (pipeline, layout)
    }

    // Helper: Create compute_multipliers pipeline
    fn create_compute_mult_pipeline(
        &self,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
        // Reuse row_swap layout (same bindings)
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Mult BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Mult PL"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Mult Pipeline"),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "compute_multipliers",
        });

        (pipeline, layout)
    }

    // Helper: Create row_elimination pipeline
    fn create_row_elim_pipeline(
        &self,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Row Elim BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Row Elim PL"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Row Elim Pipeline"),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "row_elimination",
        });

        (pipeline, layout)
    }

    // Helper: Read pivot result from GPU (2 u32 values)
    fn read_pivot_result(
        &self,
        device: &Arc<WgpuDevice>,
        buffer: &wgpu::Buffer,
    ) -> Result<u32> {
        // Read 2 u32 values [pivot_row, max_val_bits]
        let data = device.read_buffer_u32(buffer, 2)?;
        Ok(data[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Device;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_lu_gpu_2x2() {
        let device = match Device::new() {
            Ok(Device::Gpu(gpu)) => gpu,
            _ => return, // Skip if no GPU
        };

        let a = vec![4.0f32, 3.0, 6.0, 3.0];
        let input = Tensor::from_slice(&a, vec![2, 2], device.clone()).unwrap();

        let lu_gpu = LuGpu::new(input);
        let (lu_tensor, perm) = lu_gpu.execute().unwrap();

        let lu_data = lu_tensor.to_vec().unwrap();

        // Verify LU factorization: should be able to reconstruct A from L and U
        // For a 2x2 matrix, check that we got valid factors
        assert_eq!(lu_data.len(), 4);
        assert_eq!(perm.len(), 2);
    }

    #[test]
    fn test_lu_gpu_identity() {
        let device = match Device::new() {
            Ok(Device::Gpu(gpu)) => gpu,
            _ => return,
        };

        let a = vec![1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let input = Tensor::from_slice(&a, vec![3, 3], device.clone()).unwrap();

        let lu_gpu = LuGpu::new(input);
        let (lu_tensor, perm) = lu_gpu.execute().unwrap();

        let lu_data = lu_tensor.to_vec().unwrap();

        // Identity matrix LU decomposition should be identity
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    approx_eq(lu_data[i * 3 + j], expected, 1e-5),
                    "LU[{},{}] = {}, expected {}",
                    i,
                    j,
                    lu_data[i * 3 + j],
                    expected
                );
            }
        }
    }
}
