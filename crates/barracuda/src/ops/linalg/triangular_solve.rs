//! Triangular Solve - Forward/Backward Substitution - Pure WGSL
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
//! Solves triangular linear systems:
//! ```text
//! Forward:  L·x = b (lower triangular L)
//! Backward: Uᵀ·x = b (upper triangular U or transpose of lower)
//!
//! Used after Cholesky: A = L·Lᵀ
//! 1. Solve L·z = b (forward)
//! 2. Solve Lᵀ·x = z (backward)
//! Result: x solves A·x = b
//! ```
//!
//! ## Precision Support
//!
//! - `execute()` - f32 precision
//! - `TriangularSolveF64::execute()` - f64 precision (science-grade)
//! - `TriangularSolveF64::execute_transpose()` - Solve using Lᵀ (for Cholesky step 2)
//!
//! ## Use Case
//!
//! **RBF Surrogate Learning** (hotSpring physics integration):
//! - After Cholesky: K = L·Lᵀ
//! - Solve K·w = y → solve L·(Lᵀ·w) = y
//! - Step 1: L·z = y (forward) → z
//! - Step 2: Lᵀ·w = z (backward) → w
//! - Result: w are the RBF weights
//!
//! ## References
//!
//! - Golub & Van Loan, "Matrix Computations", Section 3.1
//! - Used in scipy.linalg.solve_triangular
//! - Completes the Cholesky solve pipeline

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Triangular solve operation
///
/// Solves L·x = b (forward) or Uᵀ·x = b (backward)
pub struct TriangularSolve {
    matrix: Tensor, // Triangular matrix (L or U)
    rhs: Tensor,    // Right-hand side vector b
    lower: bool,    // true for lower triangular (forward), false for upper (backward)
}

impl TriangularSolve {
    /// Create new triangular solve operation
    ///
    /// # Arguments
    /// * `matrix` - Triangular matrix [N, N]
    /// * `rhs` - Right-hand side vector [N]
    /// * `lower` - true for lower triangular (forward substitution)
    ///
    /// # Deep Debt Compliance
    /// - No hardcoded sizes (runtime N)
    /// - No unsafe blocks
    /// - Agnostic design (works with any triangular system)
    pub fn new(matrix: Tensor, rhs: Tensor, lower: bool) -> Self {
        Self { matrix, rhs, lower }
    }

    /// Create forward substitution: L·x = b
    pub fn forward(matrix: Tensor, rhs: Tensor) -> Self {
        Self::new(matrix, rhs, true)
    }

    /// Create backward substitution: Uᵀ·x = b
    pub fn backward(matrix: Tensor, rhs: Tensor) -> Self {
        Self::new(matrix, rhs, false)
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/linalg/triangular_solve.wgsl")
    }

    fn wgsl_shader_f64() -> &'static str {
        include_str!("../../shaders/linalg/triangular_solve_f64.wgsl")
    }

    /// Execute triangular solve on GPU
    ///
    /// # Returns
    /// Solution vector x
    ///
    /// # Errors
    /// - Returns error if matrix is not square
    /// - Returns error if rhs size doesn't match matrix
    /// - Returns zero vector if matrix is singular
    ///
    /// # Deep Debt Compliance
    /// - Pure WGSL execution (no CPU fallback)
    /// - Capability-based workgroup dispatch
    /// - Safe buffer management
    pub fn execute(self) -> Result<Tensor> {
        let device = self.matrix.device();
        let matrix_shape = self.matrix.shape();
        let rhs_shape = self.rhs.shape();

        // Validate square matrix
        if matrix_shape.len() != 2 || matrix_shape[0] != matrix_shape[1] {
            return Err(BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: matrix_shape.to_vec(),
            });
        }

        let n = matrix_shape[0];

        // Validate rhs is a vector of length n
        if rhs_shape.len() != 1 || rhs_shape[0] != n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n],
                actual: rhs_shape.to_vec(),
            });
        }

        // Create output buffer for solution vector x
        let solution_buffer = device.create_buffer_f32(n)?;

        // Create params buffer with matrix size and substitution type
        let is_lower = if self.lower { 1u32 } else { 0u32 };
        let params_buffer =
            device.create_uniform_buffer("TriangularSolve Params", &[n as u32, is_lower]);

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TriangularSolve BGL"),
                    entries: &[
                        // Triangular matrix (L or U)
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
                        // Right-hand side vector b
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
                        // Solution vector x
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
                        // Parameters (n, is_lower)
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TriangularSolve BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.matrix.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: solution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("TriangularSolve"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("TriangularSolve PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("TriangularSolve Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Create command encoder
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TriangularSolve Encoder"),
            });

        // Execute compute pass
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TriangularSolve Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Sequential algorithm (dependency chain)
            // Single-threaded execution (workgroup_size=1 in shader)
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let output_data = crate::utils::read_buffer(device, &solution_buffer, n)?;
        Ok(Tensor::new(output_data, vec![n], device.clone()))
    }
}

/// Tensor extension for triangular solve
impl Tensor {
    /// Solve L·x = b (forward substitution)
    ///
    /// # Arguments
    /// * `rhs` - Right-hand side vector b
    ///
    /// # Returns
    /// Solution vector x
    ///
    /// # Example
    /// ```ignore
    /// let l = tensor.cholesky()?;  // Get lower triangular L
    /// let x = l.solve_triangular_forward(&b)?;
    /// ```
    pub fn solve_triangular_forward(&self, rhs: &Tensor) -> Result<Tensor> {
        TriangularSolve::forward(self.clone(), rhs.clone()).execute()
    }

    /// Solve Uᵀ·x = b (backward substitution)
    ///
    /// # Arguments
    /// * `rhs` - Right-hand side vector b
    ///
    /// # Returns
    /// Solution vector x
    pub fn solve_triangular_backward(&self, rhs: &Tensor) -> Result<Tensor> {
        TriangularSolve::backward(self.clone(), rhs.clone()).execute()
    }

    /// Solve triangular system L·x = b or Uᵀ·x = b
    ///
    /// # Arguments
    /// * `rhs` - Right-hand side vector b
    /// * `lower` - true for lower triangular, false for upper
    pub fn solve_triangular(&self, rhs: &Tensor, lower: bool) -> Result<Tensor> {
        TriangularSolve::new(self.clone(), rhs.clone(), lower).execute()
    }
}

/// Triangular solve for f64 data (GPU)
///
/// **Deep Debt Evolution (Feb 16, 2026)**:
/// - Science-grade f64 precision
/// - Native Vulkan fp64 arithmetic
/// - WGSL as unified math language
/// - Includes transpose solve for Cholesky pipeline
pub struct TriangularSolveF64;

impl TriangularSolveF64 {
    /// Solve triangular system L·x = b or U·x = b with f64 precision
    ///
    /// # Arguments
    /// * `device` - GPU device (Arc-wrapped)
    /// * `matrix` - Triangular matrix (row-major f64)
    /// * `rhs` - Right-hand side vector b
    /// * `n` - Matrix/vector dimension
    /// * `lower` - true for lower triangular (forward), false for upper (backward)
    /// * `unit_diagonal` - true if diagonal is implicitly 1.0
    ///
    /// # Returns
    /// Solution vector x
    pub fn execute(
        device: std::sync::Arc<crate::device::WgpuDevice>,
        matrix: &[f64],
        rhs: &[f64],
        n: usize,
        lower: bool,
        unit_diagonal: bool,
    ) -> Result<Vec<f64>> {
        if matrix.len() != n * n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n * n],
                actual: vec![matrix.len()],
            });
        }
        if rhs.len() != n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n],
                actual: vec![rhs.len()],
            });
        }

        // Create buffers
        let matrix_buffer = device.create_buffer_f64(n * n)?;
        device
            .queue
            .write_buffer(&matrix_buffer, 0, bytemuck::cast_slice(matrix));

        let rhs_buffer = device.create_buffer_f64(n)?;
        device
            .queue
            .write_buffer(&rhs_buffer, 0, bytemuck::cast_slice(rhs));

        let solution_buffer = device.create_buffer_f64(n)?;

        // Params: n, is_lower, is_unit, _pad
        let is_lower = if lower { 1u32 } else { 0u32 };
        let is_unit = if unit_diagonal { 1u32 } else { 0u32 };
        let params_buffer = device.create_uniform_buffer(
            "TriangularSolve F64 Params",
            &[n as u32, is_lower, is_unit, 0u32],
        );

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TriangularSolve F64 BGL"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TriangularSolve F64 BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rhs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: solution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(
            TriangularSolve::wgsl_shader_f64(),
            Some("TriangularSolve F64"),
        );

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("TriangularSolve F64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("TriangularSolve F64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "triangular_solve_f64",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TriangularSolve F64 Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TriangularSolve F64 Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        crate::utils::read_buffer_f64(&device, &solution_buffer, n)
    }

    /// Solve L·x = b (forward substitution) with f64
    pub fn forward(
        device: std::sync::Arc<crate::device::WgpuDevice>,
        matrix: &[f64],
        rhs: &[f64],
        n: usize,
    ) -> Result<Vec<f64>> {
        Self::execute(device, matrix, rhs, n, true, false)
    }

    /// Solve U·x = b (backward substitution) with f64
    pub fn backward(
        device: std::sync::Arc<crate::device::WgpuDevice>,
        matrix: &[f64],
        rhs: &[f64],
        n: usize,
    ) -> Result<Vec<f64>> {
        Self::execute(device, matrix, rhs, n, false, false)
    }

    /// Solve Lᵀ·x = b using stored L (transpose solve)
    ///
    /// This is the second step of Cholesky solve:
    /// 1. L·z = b (forward)
    /// 2. Lᵀ·x = z (this method)
    ///
    /// The matrix is accessed as transpose internally.
    pub fn solve_transpose(
        device: std::sync::Arc<crate::device::WgpuDevice>,
        matrix: &[f64],
        rhs: &[f64],
        n: usize,
    ) -> Result<Vec<f64>> {
        if matrix.len() != n * n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n * n],
                actual: vec![matrix.len()],
            });
        }
        if rhs.len() != n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n],
                actual: vec![rhs.len()],
            });
        }

        // Create buffers
        let matrix_buffer = device.create_buffer_f64(n * n)?;
        device
            .queue
            .write_buffer(&matrix_buffer, 0, bytemuck::cast_slice(matrix));

        let rhs_buffer = device.create_buffer_f64(n)?;
        device
            .queue
            .write_buffer(&rhs_buffer, 0, bytemuck::cast_slice(rhs));

        let solution_buffer = device.create_buffer_f64(n)?;

        // Params: n, is_lower=1 (but we use transpose kernel), is_unit=0, _pad
        let params_buffer = device.create_uniform_buffer(
            "TriangularSolve Transpose F64 Params",
            &[n as u32, 1u32, 0u32, 0u32],
        );

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TriangularSolve Transpose F64 BGL"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TriangularSolve Transpose F64 BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rhs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: solution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(
            TriangularSolve::wgsl_shader_f64(),
            Some("TriangularSolve Transpose F64"),
        );

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("TriangularSolve Transpose F64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("TriangularSolve Transpose F64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "triangular_solve_transpose_f64",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TriangularSolve Transpose F64 Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TriangularSolve Transpose F64 Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        crate::utils::read_buffer_f64(&device, &solution_buffer, n)
    }

    /// Complete Cholesky solve: Given L from Cholesky(A), solve A·x = b
    ///
    /// Performs:
    /// 1. L·z = b (forward substitution)
    /// 2. Lᵀ·x = z (backward with transpose)
    ///
    /// # Arguments
    /// * `device` - GPU device (Arc-wrapped)
    /// * `l_matrix` - Lower triangular Cholesky factor L
    /// * `b` - Right-hand side vector
    /// * `n` - System dimension
    ///
    /// # Returns
    /// Solution vector x where A·x = b
    pub fn cholesky_solve(
        device: std::sync::Arc<crate::device::WgpuDevice>,
        l_matrix: &[f64],
        b: &[f64],
        n: usize,
    ) -> Result<Vec<f64>> {
        // Step 1: L·z = b (forward)
        let z = Self::forward(device.clone(), l_matrix, b, n)?;

        // Step 2: Lᵀ·x = z (transpose solve)
        Self::solve_transpose(device, l_matrix, &z, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::{get_test_device_if_f64_gpu_available, get_test_device_if_gpu_available};

    #[tokio::test]
    async fn test_forward_substitution_2x2() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Lower triangular matrix L = [[2, 0], [3, 4]]
        // Solve L·x = b where b = [6, 17]
        // Expected: x = [3, 2]
        // Verification: [2,0]*[3] + [3,4]*[2] = [6, 17] ✓
        let l_data = vec![2.0, 0.0, 3.0, 4.0];
        let b_data = vec![6.0, 17.0];

        let l = Tensor::from_vec_on(l_data, vec![2, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![2], device).await.unwrap();

        let x = l.solve_triangular_forward(&b).unwrap();
        let solution = x.to_vec().unwrap();

        // Check x ≈ [3, 2]
        assert_eq!(solution.len(), 2);
        assert!(
            (solution[0] - 3.0).abs() < 1e-5,
            "x[0] should be 3.0, got {}",
            solution[0]
        );
        assert!(
            (solution[1] - 2.0).abs() < 1e-5,
            "x[1] should be 2.0, got {}",
            solution[1]
        );
    }

    #[tokio::test]
    async fn test_backward_substitution_2x2() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Upper triangular matrix U = [[2, 3], [0, 4]]
        // Solve U·x = b where b = [12, 8]
        // Expected: x = [3, 2]
        let u_data = vec![2.0, 3.0, 0.0, 4.0];
        let b_data = vec![12.0, 8.0];

        let u = Tensor::from_vec_on(u_data, vec![2, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![2], device).await.unwrap();

        let x = u.solve_triangular_backward(&b).unwrap();
        let solution = x.to_vec().unwrap();

        // Check x ≈ [3, 2]
        assert_eq!(solution.len(), 2);
        assert!(
            (solution[0] - 3.0).abs() < 1e-5,
            "x[0] should be 3.0, got {}",
            solution[0]
        );
        assert!(
            (solution[1] - 2.0).abs() < 1e-5,
            "x[1] should be 2.0, got {}",
            solution[1]
        );
    }

    #[tokio::test]
    async fn test_cholesky_solve_pipeline() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Test complete pipeline: A·x = b
        // A = [[4, 2], [2, 3]] (SPD)
        // b = [6, 5]
        // Expected solution can be computed

        let a_data = vec![4.0, 2.0, 2.0, 3.0];
        let b_expected = vec![6.0, 5.0];

        let a = Tensor::from_vec_on(a_data.clone(), vec![2, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_expected.clone(), vec![2], device.clone())
            .await
            .unwrap();

        // Step 1: Cholesky decomposition A = L·Lᵀ
        let l = a.cholesky().unwrap();

        // Step 2: Solve L·z = b (forward)
        let z = l.solve_triangular_forward(&b).unwrap();

        // Step 3: Solve Lᵀ·x = z (backward)
        let l_t = l.transpose().unwrap();
        let x = l_t.solve_triangular_backward(&z).unwrap();

        let _solution = x.to_vec().unwrap();

        // Verify A·x ≈ b by reconstructing A from data (matmul expects 2D: [2,2] @ [2,1])
        let a_verify = Tensor::from_vec_on(a_data, vec![2, 2], device)
            .await
            .unwrap();
        let x_2d = x.unsqueeze(1).unwrap();
        let ax = a_verify.matmul(&x_2d).unwrap().squeeze().unwrap();
        let ax_data = ax.to_vec().unwrap();

        for (i, (&expected, &actual)) in b_expected.iter().zip(ax_data.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-4,
                "A·x verification failed at index {}: expected {}, got {}",
                i,
                expected,
                actual
            );
        }
    }

    // =========================================================================
    // F64 Tests — Science-grade precision
    // =========================================================================

    #[tokio::test]
    async fn test_triangular_solve_f64_forward() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };
        // L = [[2, 0], [3, 4]], b = [6, 17]
        // Solve L·x = b → x = [3, 2]
        let l: Vec<f64> = vec![2.0, 0.0, 3.0, 4.0];
        let b: Vec<f64> = vec![6.0, 17.0];

        let x = TriangularSolveF64::forward(device, &l, &b, 2).unwrap();

        assert!(
            (x[0] - 3.0).abs() < 1e-12,
            "x[0] should be 3.0, got {}",
            x[0]
        );
        assert!(
            (x[1] - 2.0).abs() < 1e-12,
            "x[1] should be 2.0, got {}",
            x[1]
        );
    }

    #[tokio::test]
    async fn test_triangular_solve_f64_backward() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };
        // U = [[2, 3], [0, 4]], b = [12, 8]
        // Solve U·x = b → x = [3, 2]
        let u: Vec<f64> = vec![2.0, 3.0, 0.0, 4.0];
        let b: Vec<f64> = vec![12.0, 8.0];

        let x = TriangularSolveF64::backward(device, &u, &b, 2).unwrap();

        assert!(
            (x[0] - 3.0).abs() < 1e-12,
            "x[0] should be 3.0, got {}",
            x[0]
        );
        assert!(
            (x[1] - 2.0).abs() < 1e-12,
            "x[1] should be 2.0, got {}",
            x[1]
        );
    }

    #[tokio::test]
    async fn test_triangular_solve_f64_cholesky_pipeline() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };
        // Complete Cholesky solve with f64 precision
        // A = [[4, 2], [2, 3]] (SPD)
        // b = [6, 5]
        // Solve A·x = b via Cholesky

        let a: Vec<f64> = vec![4.0, 2.0, 2.0, 3.0];
        let b: Vec<f64> = vec![6.0, 5.0];
        let n = 2;

        // Step 1: Cholesky decomposition
        use super::super::cholesky::CholeskyF64;
        let l = CholeskyF64::execute(device.clone(), &a, n).unwrap();

        // Step 2: Complete solve using cholesky_solve helper
        let x = TriangularSolveF64::cholesky_solve(device, &l, &b, n).unwrap();

        // Verify: A·x should equal b
        // Manual A·x multiplication
        let ax0 = a[0] * x[0] + a[1] * x[1];
        let ax1 = a[2] * x[0] + a[3] * x[1];

        assert!(
            (ax0 - b[0]).abs() < 1e-10,
            "A·x[0] should be {}, got {}",
            b[0],
            ax0
        );
        assert!(
            (ax1 - b[1]).abs() < 1e-10,
            "A·x[1] should be {}, got {}",
            b[1],
            ax1
        );
    }

    #[tokio::test]
    async fn test_triangular_solve_f64_3x3() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };
        // 3x3 lower triangular solve with f64
        // L = [[2, 0, 0], [1, 3, 0], [4, 2, 5]]
        // b = [4, 7, 28]
        // Expected x = [2, 5/3, 2.6667...]
        let l: Vec<f64> = vec![2.0, 0.0, 0.0, 1.0, 3.0, 0.0, 4.0, 2.0, 5.0];
        // Create b such that L·x = b has a known solution
        // Let x = [2, 1, 3]
        // L·x = [4, 5, 25]
        let b: Vec<f64> = vec![4.0, 5.0, 25.0];

        let x = TriangularSolveF64::forward(device, &l, &b, 3).unwrap();

        assert!(
            (x[0] - 2.0).abs() < 1e-12,
            "x[0] should be 2.0, got {}",
            x[0]
        );
        assert!(
            (x[1] - 1.0).abs() < 1e-12,
            "x[1] should be 1.0, got {}",
            x[1]
        );
        assert!(
            (x[2] - 3.0).abs() < 1e-12,
            "x[2] should be 3.0, got {}",
            x[2]
        );
    }
}
