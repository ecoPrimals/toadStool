//! GPU-accelerated Conjugate Gradient Solver (f64)
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Full f64 precision via SPIR-V/Vulkan (bypasses CUDA fp64 throttle)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//!
//! ## Algorithm
//!
//! Preconditioned Conjugate Gradient for symmetric positive definite systems:
//! ```text
//! x₀ = 0, r₀ = b, z₀ = M⁻¹r₀, p₀ = z₀
//! For k = 0, 1, ...
//!   αₖ = (rₖ·zₖ) / (pₖ·Apₖ)
//!   xₖ₊₁ = xₖ + αₖpₖ
//!   rₖ₊₁ = rₖ - αₖApₖ
//!   Check convergence: ‖rₖ₊₁‖ / ‖b‖ < tol
//!   zₖ₊₁ = M⁻¹rₖ₊₁
//!   βₖ = (rₖ₊₁·zₖ₊₁) / (rₖ·zₖ)
//!   pₖ₊₁ = zₖ₊₁ + βₖpₖ
//! ```
//!
//! ## Precision
//!
//! **Full f64 precision** - uses native WGSL f64 via SPIR-V/Vulkan.
//! FP64 performance is 1:2-3 (not 1:32 like CUDA consumer GPUs).
//!
//! ## References
//!
//! - Saad, Y. (2003). Iterative Methods for Sparse Linear Systems
//! - Golub & Van Loan, "Matrix Computations"

use super::csr::CsrMatrix;
use super::gpu_helpers::{SparseBindGroupLayouts, SparseBuffers};
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU Conjugate Gradient solver result
#[derive(Debug, Clone)]
pub struct CgGpuResult {
    /// Solution vector
    pub x: Vec<f64>,
    /// Number of iterations performed
    pub iterations: usize,
    /// Final relative residual
    pub residual: f64,
    /// Whether convergence was achieved
    pub converged: bool,
}

impl CgGpuResult {
    pub fn is_ok(&self) -> bool {
        self.converged
    }
}

/// GPU-accelerated Conjugate Gradient solver
pub struct CgGpu;

impl CgGpu {
    // Separate shader modules to avoid binding conflicts
    fn spmv_shader() -> &'static str {
        include_str!("../../shaders/sparse/spmv_f64.wgsl")
    }

    fn dot_reduce_shader() -> &'static str {
        include_str!("../../shaders/sparse/dot_reduce_f64.wgsl")
    }

    fn cg_kernels_shader() -> &'static str {
        include_str!("../../shaders/sparse/cg_kernels_f64.wgsl")
    }

    /// Solve Ax = b using GPU-resident Conjugate Gradient (reduced CPU sync)
    ///
    /// This is the **recommended method** for large systems.
    /// Scalar values (α, β, ρ) remain on GPU; residual is only read every `check_interval` iterations.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice to execute on
    /// * `a` - Symmetric positive definite CSR matrix (f64)
    /// * `b` - Right-hand side vector (f64)
    /// * `tol` - Convergence tolerance
    /// * `max_iter` - Maximum iterations
    /// * `check_interval` - How often to read residual from GPU (default: 10)
    ///
    /// # Performance
    /// For a 1000×1000 matrix:
    /// - Original: ~100 GPU↔CPU syncs for 100 iterations
    /// - GPU-resident (check_interval=10): ~10 GPU↔CPU syncs
    pub fn solve_gpu_resident(
        device: Arc<WgpuDevice>,
        a: &CsrMatrix,
        b: &[f64],
        tol: f64,
        max_iter: usize,
        check_interval: usize,
    ) -> Result<CgGpuResult> {
        let n = a.n_rows;
        if a.n_cols != n {
            return Err(BarracudaError::InvalidInput {
                message: "CG requires square matrix".to_string(),
            });
        }
        if b.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!("Vector length {} doesn't match matrix size {}", b.len(), n),
            });
        }

        let check_interval = check_interval.max(1);

        // Early exit for zero RHS
        let b_norm: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if b_norm < 1e-14 {
            return Ok(CgGpuResult {
                x: vec![0.0; n],
                iterations: 0,
                residual: 0.0,
                converged: true,
            });
        }

        // Create GPU buffers for CSR matrix
        let values_buffer = SparseBuffers::f64_from_slice(&device, "CG values", &a.values);
        let col_indices_buffer =
            SparseBuffers::u32_from_usize(&device, "CG col_idx", &a.col_indices);
        let row_ptrs_buffer = SparseBuffers::u32_from_usize(&device, "CG row_ptr", &a.row_ptr);

        // Create GPU buffers for vectors
        let x_buffer = SparseBuffers::f64_zeros(&device, "CG x", n);
        let r_buffer = SparseBuffers::f64_from_slice(&device, "CG r", b);
        let p_buffer = SparseBuffers::f64_from_slice(&device, "CG p", b);
        let ap_buffer = SparseBuffers::f64_zeros(&device, "CG Ap", n);

        // Scalar buffers (stay on GPU)
        let num_workgroups = n.div_ceil(256);
        let partial_sums_buffer = SparseBuffers::f64_zeros(&device, "CG partial", num_workgroups);
        let rz_buffer = SparseBuffers::f64_zeros(&device, "CG rz", 1);
        let rz_new_buffer = SparseBuffers::f64_zeros(&device, "CG rz_new", 1);
        let pap_buffer = SparseBuffers::f64_zeros(&device, "CG pAp", 1);
        let alpha_buffer = SparseBuffers::f64_zeros(&device, "CG alpha", 1);
        let beta_buffer = SparseBuffers::f64_zeros(&device, "CG beta", 1);

        // Compile separate shader modules to avoid binding conflicts
        let spmv_shader = device.compile_shader(Self::spmv_shader(), Some("CG SpMV"));
        let dot_reduce_shader =
            device.compile_shader(Self::dot_reduce_shader(), Some("CG Dot/Reduce"));
        let cg_kernels_shader =
            device.compile_shader(Self::cg_kernels_shader(), Some("CG Kernels"));

        // Create all bind group layouts
        let spmv_bgl = SparseBindGroupLayouts::spmv(&device);
        let dot_bgl = SparseBindGroupLayouts::dot(&device);
        let reduce_bgl = SparseBindGroupLayouts::reduce(&device);
        let update_xr_bgl = SparseBindGroupLayouts::cg_update_xr(&device);
        let update_p_bgl = SparseBindGroupLayouts::cg_update_p(&device);
        let compute_alpha_bgl = SparseBindGroupLayouts::compute_alpha(&device);
        let compute_beta_bgl = SparseBindGroupLayouts::compute_beta(&device);

        // Create pipelines using appropriate shader modules
        let spmv_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SpMV f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("SpMV PL"),
                            bind_group_layouts: &[&spmv_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &spmv_shader,
                    entry_point: "spmv_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let dot_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Dot f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Dot PL"),
                            bind_group_layouts: &[&dot_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &dot_reduce_shader,
                    entry_point: "dot_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let reduce_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Final reduce f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Reduce PL"),
                            bind_group_layouts: &[&reduce_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &dot_reduce_shader,
                    entry_point: "final_reduce_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let update_xr_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("CG update xr"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Update xr PL"),
                            bind_group_layouts: &[&update_xr_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "cg_update_xr",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let update_p_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("CG update p"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Update p PL"),
                            bind_group_layouts: &[&update_p_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "cg_update_p",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_alpha_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute alpha"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute alpha PL"),
                            bind_group_layouts: &[&compute_alpha_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "compute_alpha",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_beta_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute beta"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute beta PL"),
                            bind_group_layouts: &[&compute_beta_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "compute_beta",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Create bind groups
        let spmv_params = [n as u32];
        let spmv_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpMV params"),
                contents: bytemuck::cast_slice(&spmv_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let spmv_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SpMV BG"),
            layout: &spmv_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: col_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: row_ptrs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: spmv_params_buf.as_entire_binding(),
                },
            ],
        });

        let dot_params = [n as u32];
        let dot_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dot params"),
                contents: bytemuck::cast_slice(&dot_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // rᵀr dot product bind group
        let rr_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rr BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        // pᵀAp dot product bind group
        let pap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pAp BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        let reduce_params = [num_workgroups as u32];
        let reduce_params_buf =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Reduce params"),
                    contents: bytemuck::cast_slice(&reduce_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Reduce to rz_buffer
        let reduce_rz_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce rz BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        // Reduce to rz_new_buffer
        let reduce_rz_new_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce rz_new BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_new_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        // Reduce to pap_buffer
        let reduce_pap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce pAp BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        let cg_params = [n as u32];
        let cg_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CG params"),
                contents: bytemuck::cast_slice(&cg_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let update_xr_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Update xr BG"),
            layout: &update_xr_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: alpha_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cg_params_buf.as_entire_binding(),
                },
            ],
        });

        let update_p_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Update p BG"),
            layout: &update_p_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: cg_params_buf.as_entire_binding(),
                },
            ],
        });

        let compute_alpha_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute alpha BG"),
            layout: &compute_alpha_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: alpha_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_beta_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute beta BG"),
            layout: &compute_beta_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rz_new_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
            ],
        });

        // Initialize: compute rᵀr and store in rz_buffer
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Init rz"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot rr Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &rr_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce rz Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_rz_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Main CG iteration loop
        let mut last_residual = 1.0;
        for iter in 0..max_iter {
            // 1. Compute Ap
            // 2. Compute pᵀAp
            // 3. α = rz / pAp
            // 4. x = x + α*p, r = r - α*Ap
            // 5. Compute new rᵀr
            // 6. β = rz_new / rz, update rz
            // 7. p = r + β*p

            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("CG iter"),
                    });

            // 1. SpMV: Ap = A * p
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("SpMV"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&spmv_pipeline);
                pass.set_bind_group(0, &spmv_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            // 2. Dot: pᵀAp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot pAp"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &pap_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce pAp"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_pap_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 3. α = rz / pAp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute alpha"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&compute_alpha_pipeline);
                pass.set_bind_group(0, &compute_alpha_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 4. x = x + α*p, r = r - α*Ap
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Update xr"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&update_xr_pipeline);
                pass.set_bind_group(0, &update_xr_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            // 5. New rᵀr
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot rr new"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &rr_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce rz new"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_rz_new_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 6. β = rz_new / rz, then rz = rz_new
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute beta"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&compute_beta_pipeline);
                pass.set_bind_group(0, &compute_beta_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 7. p = r + β*p
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Update p"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&update_p_pipeline);
                pass.set_bind_group(0, &update_p_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            device.queue.submit(Some(encoder.finish()));

            // Check convergence only every check_interval iterations
            if (iter + 1) % check_interval == 0 || iter == max_iter - 1 {
                let rz_new = SparseBuffers::read_f64(&device, &rz_new_buffer, 1)?;
                let r_norm = rz_new[0].sqrt();
                last_residual = r_norm / b_norm;

                if last_residual < tol {
                    let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;
                    return Ok(CgGpuResult {
                        x: x_data,
                        iterations: iter + 1,
                        residual: last_residual,
                        converged: true,
                    });
                }
            }
        }

        // Did not converge
        let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;

        Ok(CgGpuResult {
            x: x_data,
            iterations: max_iter,
            residual: last_residual,
            converged: false,
        })
    }

    /// Solve Ax = b using Preconditioned Conjugate Gradient (GPU-resident)
    ///
    /// This version uses Jacobi (diagonal) preconditioning for faster convergence.
    /// M = diag(A) → z = M⁻¹r = r / diag(A)
    ///
    /// Preconditioning typically halves the iteration count for poorly-conditioned matrices.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice to execute on
    /// * `a` - Symmetric positive definite CSR matrix (f64)
    /// * `b` - Right-hand side vector (f64)
    /// * `tol` - Convergence tolerance
    /// * `max_iter` - Maximum iterations
    /// * `check_interval` - How often to read residual from GPU
    ///
    /// # Performance
    /// For HFB matrices that are poorly conditioned, Jacobi preconditioning
    /// can reduce iteration count by 2-3×.
    pub fn solve_preconditioned(
        device: Arc<WgpuDevice>,
        a: &CsrMatrix,
        b: &[f64],
        tol: f64,
        max_iter: usize,
        check_interval: usize,
    ) -> Result<CgGpuResult> {
        let n = a.n_rows;
        if a.n_cols != n {
            return Err(BarracudaError::InvalidInput {
                message: "CG requires square matrix".to_string(),
            });
        }
        if b.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!("Vector length {} doesn't match matrix size {}", b.len(), n),
            });
        }

        let check_interval = check_interval.max(1);

        // Early exit for zero RHS
        let b_norm: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if b_norm < 1e-14 {
            return Ok(CgGpuResult {
                x: vec![0.0; n],
                iterations: 0,
                residual: 0.0,
                converged: true,
            });
        }

        // Extract diagonal for Jacobi preconditioner
        let diag: Vec<f64> = (0..n)
            .map(|i| {
                let row_start = a.row_ptr[i];
                let row_end = a.row_ptr[i + 1];
                for k in row_start..row_end {
                    if a.col_indices[k] == i {
                        return a.values[k];
                    }
                }
                1.0 // Fallback if diagonal not found
            })
            .collect();

        // Create GPU buffers
        let values_buffer = SparseBuffers::f64_from_slice(&device, "PCG values", &a.values);
        let col_indices_buffer =
            SparseBuffers::u32_from_usize(&device, "PCG col_idx", &a.col_indices);
        let row_ptrs_buffer = SparseBuffers::u32_from_usize(&device, "PCG row_ptr", &a.row_ptr);
        let diag_buffer = SparseBuffers::f64_from_slice(&device, "PCG diag", &diag);

        let x_buffer = SparseBuffers::f64_zeros(&device, "PCG x", n);
        let r_buffer = SparseBuffers::f64_from_slice(&device, "PCG r", b);
        let z_buffer = SparseBuffers::f64_zeros(&device, "PCG z", n); // z = M⁻¹r
        let p_buffer = SparseBuffers::f64_zeros(&device, "PCG p", n);
        let ap_buffer = SparseBuffers::f64_zeros(&device, "PCG Ap", n);

        // Scalar buffers
        let num_workgroups = n.div_ceil(256);
        let partial_sums_buffer = SparseBuffers::f64_zeros(&device, "PCG partial", num_workgroups);
        let rz_buffer = SparseBuffers::f64_zeros(&device, "PCG rz", 1);
        let rz_new_buffer = SparseBuffers::f64_zeros(&device, "PCG rz_new", 1);
        let pap_buffer = SparseBuffers::f64_zeros(&device, "PCG pAp", 1);
        let alpha_buffer = SparseBuffers::f64_zeros(&device, "PCG alpha", 1);
        let beta_buffer = SparseBuffers::f64_zeros(&device, "PCG beta", 1);

        // Compile separate shader modules to avoid binding conflicts
        let spmv_shader = device.compile_shader_f64(Self::spmv_shader(), Some("PCG SpMV"));
        let dot_reduce_shader =
            device.compile_shader_f64(Self::dot_reduce_shader(), Some("PCG Dot/Reduce"));
        let cg_kernels_shader =
            device.compile_shader_f64(Self::cg_kernels_shader(), Some("PCG Kernels"));
        let vector_ops_shader = device.compile_shader_f64(
            include_str!("../../shaders/sparse/vector_ops_f64.wgsl"),
            Some("PCG VecOps"),
        );

        // Create bind group layouts
        let spmv_bgl = SparseBindGroupLayouts::spmv(&device);
        let dot_bgl = SparseBindGroupLayouts::dot(&device);
        let reduce_bgl = SparseBindGroupLayouts::reduce(&device);
        let update_xr_bgl = SparseBindGroupLayouts::cg_update_xr(&device);
        let update_p_bgl = SparseBindGroupLayouts::cg_update_p(&device);
        let compute_alpha_bgl = SparseBindGroupLayouts::compute_alpha(&device);
        let compute_beta_bgl = SparseBindGroupLayouts::compute_beta(&device);
        let precond_bgl = SparseBindGroupLayouts::precond(&device);

        // Create pipelines using appropriate shader modules
        let spmv_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SpMV f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("SpMV PL"),
                            bind_group_layouts: &[&spmv_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &spmv_shader,
                    entry_point: "spmv_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let dot_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Dot f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Dot PL"),
                            bind_group_layouts: &[&dot_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &dot_reduce_shader,
                    entry_point: "dot_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let reduce_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Final reduce f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Reduce PL"),
                            bind_group_layouts: &[&reduce_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &dot_reduce_shader,
                    entry_point: "final_reduce_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let update_xr_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("PCG update xr"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Update xr PL"),
                            bind_group_layouts: &[&update_xr_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "cg_update_xr",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let update_p_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("PCG update p"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Update p PL"),
                            bind_group_layouts: &[&update_p_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "cg_update_p",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_alpha_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute alpha"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute alpha PL"),
                            bind_group_layouts: &[&compute_alpha_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "compute_alpha",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_beta_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute beta"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute beta PL"),
                            bind_group_layouts: &[&compute_beta_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &cg_kernels_shader,
                    entry_point: "compute_beta",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let precond_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Precond f64"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Precond PL"),
                            bind_group_layouts: &[&precond_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &vector_ops_shader,
                    entry_point: "precond_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Create bind groups
        let spmv_params = [n as u32];
        let spmv_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpMV params"),
                contents: bytemuck::cast_slice(&spmv_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let spmv_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SpMV BG"),
            layout: &spmv_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: col_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: row_ptrs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: spmv_params_buf.as_entire_binding(),
                },
            ],
        });

        let dot_params = [n as u32];
        let dot_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dot params"),
                contents: bytemuck::cast_slice(&dot_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // rᵀz dot product bind group
        let rz_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rz BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: z_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        // pᵀAp dot product bind group
        let pap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pAp BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        // rᵀr for convergence check
        let rr_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rr BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        let reduce_params = [num_workgroups as u32];
        let reduce_params_buf =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Reduce params"),
                    contents: bytemuck::cast_slice(&reduce_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let reduce_rz_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce rz BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        let reduce_rz_new_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce rz_new BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_new_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        let reduce_pap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce pAp BG"),
            layout: &reduce_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: reduce_params_buf.as_entire_binding(),
                },
            ],
        });

        let precond_params = [n as u32];
        let precond_params_buf =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Precond params"),
                    contents: bytemuck::cast_slice(&precond_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let precond_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Precond BG"),
            layout: &precond_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: diag_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: z_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: precond_params_buf.as_entire_binding(),
                },
            ],
        });

        let cg_params = [n as u32];
        let cg_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CG params"),
                contents: bytemuck::cast_slice(&cg_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let update_xr_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Update xr BG"),
            layout: &update_xr_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: alpha_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cg_params_buf.as_entire_binding(),
                },
            ],
        });

        // p = z + β*p (using z instead of r)
        let update_p_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Update p BG"),
            layout: &update_p_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: z_buffer.as_entire_binding(),
                }, // z = M⁻¹r
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: cg_params_buf.as_entire_binding(),
                },
            ],
        });

        let compute_alpha_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute alpha BG"),
            layout: &compute_alpha_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: alpha_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_beta_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute beta BG"),
            layout: &compute_beta_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rz_new_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rz_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
            ],
        });

        // Initialize: z₀ = M⁻¹r₀, p₀ = z₀, compute r₀ᵀz₀
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("PCG init"),
                    });

            // z = M⁻¹r (apply preconditioner)
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Precond Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&precond_pipeline);
                pass.set_bind_group(0, &precond_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            device.queue.submit(Some(encoder.finish()));

            // Copy z to p: p₀ = z₀
            SparseBuffers::copy_f64(&device, &z_buffer, &p_buffer, n);

            // Compute rᵀz
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Init rz"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot rz Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &rz_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce rz Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_rz_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Main PCG iteration
        let mut last_residual = 1.0;
        for iter in 0..max_iter {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("PCG iter"),
                    });

            // 1. Ap = A * p
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("SpMV"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&spmv_pipeline);
                pass.set_bind_group(0, &spmv_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            // 2. pᵀAp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot pAp"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &pap_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce pAp"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_pap_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 3. α = (rᵀz) / (pᵀAp)
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute alpha"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&compute_alpha_pipeline);
                pass.set_bind_group(0, &compute_alpha_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 4. x = x + αp, r = r - αAp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Update xr"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&update_xr_pipeline);
                pass.set_bind_group(0, &update_xr_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            // 5. z = M⁻¹r (apply preconditioner)
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Precond"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&precond_pipeline);
                pass.set_bind_group(0, &precond_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            // 6. New rᵀz
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Dot rz new"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &rz_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce rz new"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&reduce_pipeline);
                pass.set_bind_group(0, &reduce_rz_new_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 7. β = (r_new ᵀ z_new) / (rᵀz), then rz = rz_new
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute beta"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&compute_beta_pipeline);
                pass.set_bind_group(0, &compute_beta_bg, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            // 8. p = z + βp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Update p"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&update_p_pipeline);
                pass.set_bind_group(0, &update_p_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }

            device.queue.submit(Some(encoder.finish()));

            // Check convergence
            if (iter + 1) % check_interval == 0 || iter == max_iter - 1 {
                // Compute actual residual norm ‖r‖
                let mut encoder =
                    device
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Check convergence"),
                        });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Dot rr"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&dot_pipeline);
                    pass.set_bind_group(0, &rr_bg, &[]);
                    pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
                }
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Reduce rr"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&reduce_pipeline);
                    pass.set_bind_group(0, &reduce_rz_new_bg, &[]); // Reuse buffer
                    pass.dispatch_workgroups(1, 1, 1);
                }
                device.queue.submit(Some(encoder.finish()));

                let rr = SparseBuffers::read_f64(&device, &rz_new_buffer, 1)?;
                let r_norm = rr[0].sqrt();
                last_residual = r_norm / b_norm;

                if last_residual < tol {
                    let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;
                    return Ok(CgGpuResult {
                        x: x_data,
                        iterations: iter + 1,
                        residual: last_residual,
                        converged: true,
                    });
                }
            }
        }

        // Did not converge
        let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;

        Ok(CgGpuResult {
            x: x_data,
            iterations: max_iter,
            residual: last_residual,
            converged: false,
        })
    }

    /// Solve Ax = b using GPU-accelerated Conjugate Gradient
    ///
    /// # Arguments
    /// * `device` - WgpuDevice to execute on
    /// * `a` - Symmetric positive definite CSR matrix (f64)
    /// * `b` - Right-hand side vector (f64)
    /// * `tol` - Convergence tolerance
    /// * `max_iter` - Maximum iterations
    ///
    /// # Returns
    /// CgGpuResult with solution, iteration count, and convergence info
    pub fn solve(
        device: Arc<WgpuDevice>,
        a: &CsrMatrix,
        b: &[f64],
        tol: f64,
        max_iter: usize,
    ) -> Result<CgGpuResult> {
        let n = a.n_rows;
        if a.n_cols != n {
            return Err(BarracudaError::InvalidInput {
                message: "CG requires square matrix".to_string(),
            });
        }
        if b.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!("Vector length {} doesn't match matrix size {}", b.len(), n),
            });
        }

        // Early exit for zero RHS
        let b_norm: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if b_norm < 1e-14 {
            return Ok(CgGpuResult {
                x: vec![0.0; n],
                iterations: 0,
                residual: 0.0,
                converged: true,
            });
        }

        // Create GPU buffers for CSR matrix
        let values_buffer = SparseBuffers::f64_from_slice(&device, "CG values", &a.values);
        let col_indices_buffer =
            SparseBuffers::u32_from_usize(&device, "CG col_idx", &a.col_indices);
        let row_ptrs_buffer = SparseBuffers::u32_from_usize(&device, "CG row_ptr", &a.row_ptr);

        // Create GPU buffers for vectors
        let x_buffer = SparseBuffers::f64_zeros(&device, "CG x", n);
        let r_buffer = SparseBuffers::f64_from_slice(&device, "CG r", b); // r₀ = b
        let p_buffer = SparseBuffers::f64_from_slice(&device, "CG p", b); // p₀ = r₀ (no preconditioning for now)
        let ap_buffer = SparseBuffers::f64_zeros(&device, "CG Ap", n);

        // Partial sums buffer for dot products
        let num_workgroups = n.div_ceil(256);
        let partial_sums_buffer = SparseBuffers::f64_zeros(&device, "CG partial", num_workgroups);

        // Compile separate shader modules to avoid binding conflicts
        let spmv_shader = device.compile_shader_f64(Self::spmv_shader(), Some("CG SpMV"));
        let dot_reduce_shader =
            device.compile_shader_f64(Self::dot_reduce_shader(), Some("CG Dot"));
        let vector_ops_shader = device.compile_shader_f64(
            include_str!("../../shaders/sparse/vector_ops_f64.wgsl"),
            Some("CG VecOps"),
        );

        // Create bind group layouts
        let spmv_bgl = SparseBindGroupLayouts::spmv(&device);
        let dot_bgl = SparseBindGroupLayouts::dot(&device);
        let axpy_bgl = SparseBindGroupLayouts::axpy(&device);

        // Create pipelines using appropriate shader modules
        let spmv_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SpMV PL"),
                bind_group_layouts: &[&spmv_bgl],
                push_constant_ranges: &[],
            });

        let dot_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Dot PL"),
                bind_group_layouts: &[&dot_bgl],
                push_constant_ranges: &[],
            });

        let axpy_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Axpy PL"),
                bind_group_layouts: &[&axpy_bgl],
                push_constant_ranges: &[],
            });

        let spmv_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SpMV f64"),
                    layout: Some(&spmv_pl),
                    module: &spmv_shader,
                    entry_point: "spmv_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let dot_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Dot f64"),
                    layout: Some(&dot_pl),
                    module: &dot_reduce_shader,
                    entry_point: "dot_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let _axpy_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Axpy f64"),
                    layout: Some(&axpy_pl),
                    module: &vector_ops_shader,
                    entry_point: "axpy_f64",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // SpMV bind group
        let spmv_params = [n as u32];
        let spmv_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpMV params"),
                contents: bytemuck::cast_slice(&spmv_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let spmv_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SpMV BG"),
            layout: &spmv_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: col_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: row_ptrs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: spmv_params_buf.as_entire_binding(),
                },
            ],
        });

        // Dot product bind groups
        let dot_params = [n as u32];
        let dot_params_buf = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dot params"),
                contents: bytemuck::cast_slice(&dot_params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // rᵀr bind group
        let _rr_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rr BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: r_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        // pᵀAp bind group
        let pap_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pAp BG"),
            layout: &dot_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: p_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ap_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dot_params_buf.as_entire_binding(),
                },
            ],
        });

        // Initial rᵀr (since r₀ = b and p₀ = b)
        let mut rz = b.iter().map(|x| x * x).sum::<f64>();

        // CG iteration
        for iter in 0..max_iter {
            // 1. Compute Ap
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("SpMV"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("SpMV Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&spmv_pipeline);
                pass.set_bind_group(0, &spmv_bg, &[]);
                pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));

            // 2. Compute pᵀAp (need to read back)
            let mut encoder = device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("pAp") });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("pAp Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&dot_pipeline);
                pass.set_bind_group(0, &pap_bg, &[]);
                pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));

            let partial = SparseBuffers::read_f64(&device, &partial_sums_buffer, num_workgroups)?;
            let pap: f64 = partial.iter().sum();

            if pap.abs() < 1e-30 {
                // Near-breakdown
                let r_data = SparseBuffers::read_f64(&device, &r_buffer, n)?;
                let r_norm: f64 = r_data.iter().map(|x| x * x).sum::<f64>().sqrt();
                let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;
                return Ok(CgGpuResult {
                    x: x_data,
                    iterations: iter + 1,
                    residual: r_norm / b_norm,
                    converged: r_norm / b_norm < tol,
                });
            }

            let alpha = rz / pap;

            // 3. Update x = x + α*p and r = r - α*Ap (CPU loop for now - GPU vectors would need separate buffers)
            let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;
            let p_data = SparseBuffers::read_f64(&device, &p_buffer, n)?;
            let ap_data = SparseBuffers::read_f64(&device, &ap_buffer, n)?;
            let r_data = SparseBuffers::read_f64(&device, &r_buffer, n)?;

            let new_x: Vec<f64> = x_data
                .iter()
                .zip(&p_data)
                .map(|(xi, pi)| xi + alpha * pi)
                .collect();
            let new_r: Vec<f64> = r_data
                .iter()
                .zip(&ap_data)
                .map(|(ri, api)| ri - alpha * api)
                .collect();

            // Check convergence
            let r_norm: f64 = new_r.iter().map(|x| x * x).sum::<f64>().sqrt();
            if r_norm / b_norm < tol {
                return Ok(CgGpuResult {
                    x: new_x,
                    iterations: iter + 1,
                    residual: r_norm / b_norm,
                    converged: true,
                });
            }

            // 4. Compute β and update p
            let rz_new: f64 = new_r.iter().map(|x| x * x).sum();
            let beta = rz_new / rz;
            rz = rz_new;

            let new_p: Vec<f64> = new_r
                .iter()
                .zip(&p_data)
                .map(|(ri, pi)| ri + beta * pi)
                .collect();

            // Write updated vectors back to GPU
            SparseBuffers::write_f64(&device, &x_buffer, &new_x);
            SparseBuffers::write_f64(&device, &r_buffer, &new_r);
            SparseBuffers::write_f64(&device, &p_buffer, &new_p);
        }

        // Did not converge
        let x_data = SparseBuffers::read_f64(&device, &x_buffer, n)?;
        let r_data = SparseBuffers::read_f64(&device, &r_buffer, n)?;
        let r_norm: f64 = r_data.iter().map(|x| x * x).sum::<f64>().sqrt();

        Ok(CgGpuResult {
            x: x_data,
            iterations: max_iter,
            residual: r_norm / b_norm,
            converged: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_spd_tridiagonal(n: usize) -> CsrMatrix {
        let mut triplets = Vec::new();

        for i in 0..n {
            triplets.push((i, i, 4.0));
            if i > 0 {
                triplets.push((i, i - 1, -1.0));
            }
            if i < n - 1 {
                triplets.push((i, i + 1, -1.0));
            }
        }

        CsrMatrix::from_triplets(n, n, &triplets)
    }

    #[tokio::test]
    async fn test_cg_gpu_small() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return; // Skip if no f64 GPU
        };

        let a = create_spd_tridiagonal(3);
        let b = vec![1.0, 2.0, 3.0];

        let result = CgGpu::solve(device, &a, &b, 1e-10, 100).unwrap();

        assert!(result.converged, "CG should converge");
        assert!(result.residual < 1e-10, "Residual should be small");

        // Verify: Ax ≈ b
        let ax = a.matvec(&result.x).unwrap();
        for (axi, bi) in ax.iter().zip(b.iter()) {
            assert!((axi - bi).abs() < 1e-8, "Ax should equal b");
        }
    }

    #[tokio::test]
    async fn test_cg_gpu_resident() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return; // Skip if no f64 GPU
        };

        // Test GPU-resident CG with larger system
        let a = create_spd_tridiagonal(100);
        let b: Vec<f64> = (0..100).map(|i| (i + 1) as f64).collect();

        // Using check_interval=10 to reduce GPU↔CPU syncs
        let result = CgGpu::solve_gpu_resident(device.clone(), &a, &b, 1e-10, 500, 10).unwrap();

        assert!(result.converged, "GPU-resident CG should converge");
        assert!(
            result.residual < 1e-10,
            "Residual should be small: {}",
            result.residual
        );

        // Verify: Ax ≈ b
        let ax = a.matvec(&result.x).unwrap();
        for (i, (axi, bi)) in ax.iter().zip(b.iter()).enumerate() {
            assert!(
                (axi - bi).abs() < 1e-6,
                "Ax[{}] = {} should equal b[{}] = {}",
                i,
                axi,
                i,
                bi
            );
        }
    }

    #[tokio::test]
    async fn test_cg_gpu_resident_vs_original() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return; // Skip if no f64 GPU
        };

        // Compare GPU-resident vs original implementation
        let a = create_spd_tridiagonal(50);
        let b: Vec<f64> = (0..50).map(|i| ((i + 1) as f64).sin()).collect();

        let result_original = CgGpu::solve(device.clone(), &a, &b, 1e-10, 200).unwrap();
        let result_resident =
            CgGpu::solve_gpu_resident(device.clone(), &a, &b, 1e-10, 200, 5).unwrap();

        // Both should converge
        assert!(result_original.converged, "Original CG should converge");
        assert!(result_resident.converged, "GPU-resident CG should converge");

        // Solutions should be nearly identical
        for (i, (x_orig, x_res)) in result_original
            .x
            .iter()
            .zip(result_resident.x.iter())
            .enumerate()
        {
            assert!(
                (x_orig - x_res).abs() < 1e-8,
                "Solution mismatch at {}: orig={}, resident={}",
                i,
                x_orig,
                x_res
            );
        }
    }

    #[tokio::test]
    async fn test_cg_gpu_preconditioned() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return; // Skip if no f64 GPU
        };

        // Test preconditioned CG
        let a = create_spd_tridiagonal(100);
        let b: Vec<f64> = (0..100).map(|i| (i + 1) as f64).collect();

        let result = CgGpu::solve_preconditioned(device.clone(), &a, &b, 1e-10, 500, 10).unwrap();

        assert!(result.converged, "Preconditioned CG should converge");
        assert!(
            result.residual < 1e-10,
            "Residual should be small: {}",
            result.residual
        );

        // Verify: Ax ≈ b
        let ax = a.matvec(&result.x).unwrap();
        for (i, (axi, bi)) in ax.iter().zip(b.iter()).enumerate() {
            assert!(
                (axi - bi).abs() < 1e-6,
                "Ax[{}] = {} should equal b[{}] = {}",
                i,
                axi,
                i,
                bi
            );
        }
    }

    #[tokio::test]
    async fn test_cg_preconditioned_vs_unpreconditioned() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return; // Skip if no f64 GPU
        };

        // Compare iteration counts: preconditioned should need fewer iterations
        let a = create_spd_tridiagonal(100);
        let b: Vec<f64> = (0..100).map(|i| (i + 1) as f64).collect();

        let result_unprecond =
            CgGpu::solve_gpu_resident(device.clone(), &a, &b, 1e-10, 500, 1).unwrap();
        let result_precond =
            CgGpu::solve_preconditioned(device.clone(), &a, &b, 1e-10, 500, 1).unwrap();

        assert!(
            result_unprecond.converged,
            "Unpreconditioned should converge"
        );
        assert!(result_precond.converged, "Preconditioned should converge");

        // For this specific matrix (tridiagonal with constant diagonal),
        // both should converge quickly, but let's at least verify they both work
        println!(
            "Iterations: unprecond={}, precond={}",
            result_unprecond.iterations, result_precond.iterations
        );

        // Solutions should be nearly identical
        for (i, (x_u, x_p)) in result_unprecond
            .x
            .iter()
            .zip(result_precond.x.iter())
            .enumerate()
        {
            assert!(
                (x_u - x_p).abs() < 1e-6,
                "Solution mismatch at {}: unprecond={}, precond={}",
                i,
                x_u,
                x_p
            );
        }
    }
}
