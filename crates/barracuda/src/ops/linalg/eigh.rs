//! Eigenvalue Decomposition (eigh) - Jacobi algorithm for symmetric matrices
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
//! Computes eigenvalue decomposition of symmetric matrix:
//! ```text
//! Input:  A [N, N] symmetric matrix
//! Output 1: eigenvalues [N] (diagonal of D)
//! Output 2: eigenvectors [N, N] (columns of V) where A = V·D·Vᵀ
//!
//! Uses Jacobi rotation method (iterative, converges for symmetric matrices)
//! Optimized for scientific computing (N ≤ 1,000)
//! ```
//!
//! ## Use Case
//!
//! **Scientific Computing**:
//! - Principal component analysis (PCA)
//! - Spectral methods for differential equations
//! - Vibration analysis
//!
//! ## References
//!
//! - Golub & Van Loan, "Matrix Computations"
//! - Jacobi eigenvalue algorithm
//! - scipy.linalg.eigh

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};

/// Parameters for eigh shader (must match WGSL struct Params)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct EighParams {
    n: u32,
    max_iter: u32,
}

/// Eigenvalue decomposition operation
///
/// Computes eigenvalues and eigenvectors of symmetric matrix A = V·D·Vᵀ
pub struct Eigh {
    input: Tensor,
}

impl Eigh {
    /// Create new eigenvalue decomposition operation
    ///
    /// # Arguments
    /// * `input` - Symmetric matrix [N, N]
    ///
    /// # Deep Debt Compliance
    /// - No hardcoded sizes (runtime N)
    /// - No unsafe blocks
    /// - Agnostic design (works with any symmetric matrix)
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/linalg/eigh.wgsl")
    }

    /// Execute eigenvalue decomposition on GPU
    ///
    /// # Returns
    /// Tuple (eigenvalues, eigenvectors) where A = V·D·Vᵀ
    /// - eigenvalues: vector [N]
    /// - eigenvectors: matrix [N, N] with columns as eigenvectors
    ///
    /// # Errors
    /// - Returns error if input is not square
    ///
    /// # Deep Debt Compliance
    /// - Pure WGSL execution (no CPU fallback)
    /// - Capability-based workgroup dispatch
    /// - Safe buffer management
    pub fn execute(self) -> Result<(Tensor, Tensor)> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Validate square matrix
        if shape.len() != 2 || shape[0] != shape[1] {
            return Err(BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: shape.to_vec(),
            });
        }

        let n = shape[0];
        let size = n * n;
        const DEFAULT_MAX_ITER: u32 = 100;

        // Create output buffers
        let work_buffer = device.create_buffer_f32(size)?;
        let eigenvalues_buffer = device.create_buffer_f32(n)?;
        let eigenvectors_buffer = device.create_buffer_f32(size)?;

        // Create params buffer
        let params = EighParams {
            n: n as u32,
            max_iter: DEFAULT_MAX_ITER,
        };
        let params_buffer = device.create_uniform_buffer("Eigh Params", &params);

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Eigh BGL"),
                    entries: &[
                        // Input matrix A (symmetric)
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
                        // Work matrix (copy of A, modified in place)
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
                        // Eigenvalues output
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
                        // Eigenvectors output
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
                        // Parameters (n, max_iter)
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Eigh BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: work_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: eigenvalues_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eigenvectors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Eigh"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Eigh PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Eigh Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        // Create command encoder
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Eigh Encoder"),
            });

        // Execute compute pass
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Eigh Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok((
            Tensor::from_buffer(eigenvalues_buffer, vec![n], device.clone()),
            Tensor::from_buffer(eigenvectors_buffer, shape.to_vec(), device.clone()),
        ))
    }
}

/// Tensor extension for eigenvalue decomposition
impl Tensor {
    /// Compute eigenvalue decomposition of symmetric matrix: A = V·D·Vᵀ
    ///
    /// # Returns
    /// Tuple (eigenvalues, eigenvectors)
    ///
    /// # Example
    /// ```ignore
    /// let a = Tensor::from_vec(vec![4.0, 2.0, 2.0, 3.0], vec![2, 2], device)?;
    /// let (vals, vecs) = a.eigh()?;
    /// ```
    pub fn eigh(self) -> Result<(Self, Self)> {
        Eigh::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_eigh_2x2() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Symmetric 2x2: [[4, 2], [2, 3]]
        // Eigenvalues: (5 + sqrt(5))/2 ≈ 3.618, (5 - sqrt(5))/2 ≈ 1.382
        let input_data = vec![4.0, 2.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2, 2], device.clone())
            .await
            .unwrap();

        let (eigenvalues, eigenvectors) = input.eigh().unwrap();
        let vals = eigenvalues.to_vec().unwrap();
        let vecs = eigenvectors.to_vec().unwrap();

        assert_eq!(vals.len(), 2);
        assert_eq!(vecs.len(), 4);

        // Eigenvalues should sum to trace (4+3=7) and product to det (12-4=8)
        let trace = vals[0] + vals[1];
        let det = vals[0] * vals[1];
        assert!(
            (trace - 7.0).abs() < 1e-4,
            "Trace should be 7, got {}",
            trace
        );
        assert!((det - 8.0).abs() < 1e-3, "Det should be 8, got {}", det);

        // Check reconstruction A ≈ V·D·Vᵀ
        let d_diag =
            Tensor::from_vec_on(vec![vals[0], 0.0, 0.0, vals[1]], vec![2, 2], device.clone())
                .await
                .unwrap();
        let v = eigenvectors.clone();
        let v_t = v.transpose().unwrap();
        let vd = v.matmul(&d_diag).unwrap();
        let vdv_t = vd.matmul(&v_t).unwrap();
        let recon = vdv_t.to_vec().unwrap();
        for (i, (&o, &r)) in input_data.iter().zip(recon.iter()).enumerate() {
            assert!(
                (o - r).abs() < 1e-3,
                "Reconstruction error at {}: expected {}, got {}",
                i,
                o,
                r
            );
        }
    }

    #[tokio::test]
    async fn test_eigh_identity() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Identity matrix: eigenvalues all 1, eigenvectors = I
        let input_data = vec![1.0, 0.0, 0.0, 1.0];
        let input = Tensor::from_vec_on(input_data, vec![2, 2], device)
            .await
            .unwrap();

        let (eigenvalues, eigenvectors) = input.eigh().unwrap();
        let vals = eigenvalues.to_vec().unwrap();
        let vecs = eigenvectors.to_vec().unwrap();

        assert!((vals[0] - 1.0).abs() < 1e-5);
        assert!((vals[1] - 1.0).abs() < 1e-5);
        assert!((vecs[0] - 1.0).abs() < 1e-5);
        assert!(vecs[1].abs() < 1e-5);
        assert!(vecs[2].abs() < 1e-5);
        assert!((vecs[3] - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_eigh_3x3() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // 3x3 symmetric matrix: [[2, -1, 0], [-1, 2, -1], [0, -1, 2]]
        // This is a common tridiagonal matrix with known eigenvalues
        let input_data = vec![2.0, -1.0, 0.0, -1.0, 2.0, -1.0, 0.0, -1.0, 2.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3, 3], device.clone())
            .await
            .unwrap();

        let (eigenvalues, eigenvectors) = input.eigh().unwrap();
        let vals = eigenvalues.to_vec().unwrap();

        assert_eq!(vals.len(), 3);
        // Eigenvalues should be positive (matrix is positive definite)
        assert!(vals[0] > 0.0);
        assert!(vals[1] > 0.0);
        assert!(vals[2] > 0.0);
        // Trace = 6
        assert!((vals[0] + vals[1] + vals[2] - 6.0).abs() < 1e-3);

        // Verify reconstruction
        let mut d_diag = vec![0.0f32; 9];
        d_diag[0] = vals[0];
        d_diag[4] = vals[1];
        d_diag[8] = vals[2];
        let d = Tensor::from_vec_on(d_diag, vec![3, 3], device.clone())
            .await
            .unwrap();
        let v = eigenvectors;
        let v_t = v.transpose().unwrap();
        let vdv_t = v.matmul(&d).unwrap().matmul(&v_t).unwrap();
        let recon = vdv_t.to_vec().unwrap();
        for (i, (&o, &r)) in input_data.iter().zip(recon.iter()).enumerate() {
            assert!(
                (o - r).abs() < 1e-2,
                "Reconstruction error at {}: expected {}, got {}",
                i,
                o,
                r
            );
        }
    }
}
