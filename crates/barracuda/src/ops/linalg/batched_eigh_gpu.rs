//! Batched Eigenvalue Decomposition (eigh) - GPU-Accelerated Implementation (f64)
//!
//! Processes multiple symmetric matrices simultaneously.
//! **Use case**: HFB Hamiltonian diagonalization (52 nuclei, 20-50 dim each)
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Full f64 precision via SPIR-V/Vulkan (bypasses CUDA fp64 throttle)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Batched parallel processing
//! - ✅ Runtime-configured matrix size and batch size
//!
//! ## Algorithm
//!
//! Jacobi eigenvalue algorithm for symmetric matrices, batched:
//! ```text
//! Input:  A_batch [batch_size × N × N] symmetric matrices
//! Output 1: eigenvalues_batch [batch_size × N]
//! Output 2: eigenvectors_batch [batch_size × N × N]
//!
//! All matrices processed in parallel on GPU.
//! Each workgroup handles one matrix from the batch.
//! ```
//!
//! ## Performance Notes
//!
//! - For 52 matrices of size 30×30: single GPU dispatch processes all
//! - Eliminates CPU loop overhead vs sequential single-matrix calls
//! - Full f64 precision maintained throughout
//!
//! ## References
//!
//! - Golub & Van Loan, "Matrix Computations", Section 8.4
//! - Demmel & Veselic (1992), "Jacobi's Method is More Accurate than QR"
//! - hotSpring HFB Hamiltonian requirements

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for batched eigh shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BatchedEighParams {
    n: u32,
    batch_size: u32,
    max_sweeps: u32,
    _pad: u32,
}

/// Parameters for parallel sweep operations
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ParallelSweepParams {
    n: u32,
    batch_size: u32,
    current_p: u32,
    current_q: u32,
}

/// GPU-accelerated batched eigenvalue decomposition
///
/// Computes eigenvalue decomposition for multiple symmetric matrices simultaneously:
/// A_i = V_i · D_i · V_i^T for all i in batch
pub struct BatchedEighGpu;

impl BatchedEighGpu {
    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/linalg/batched_eigh_f64.wgsl")
    }

    fn single_dispatch_shader() -> &'static str {
        include_str!("../../shaders/linalg/batched_eigh_single_dispatch_f64.wgsl")
    }

    /// Execute batched eigenvalue decomposition on GPU with full f64 precision
    ///
    /// This processes multiple symmetric matrices in parallel, ideal for HFB
    /// Hamiltonian diagonalization where 52 nuclei need simultaneous eigensolves.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice to execute on
    /// * `data` - Packed matrices [batch_size × n × n] in row-major order (f64)
    /// * `n` - Matrix dimension (same for all matrices)
    /// * `batch_size` - Number of matrices
    /// * `max_sweeps` - Maximum Jacobi sweeps (default: 30)
    ///
    /// # Returns
    /// Tuple (eigenvalues, eigenvectors) where:
    /// - eigenvalues: [batch_size × n] f64
    /// - eigenvectors: [batch_size × n × n] f64
    ///
    /// # Example
    /// ```ignore
    /// // 52 matrices of 30×30 dimension
    /// let batch_size = 52;
    /// let n = 30;
    /// let data: Vec<f64> = /* packed matrices */;
    /// let (eigenvalues, eigenvectors) = BatchedEighGpu::execute_f64(
    ///     device, &data, n, batch_size, 30
    /// )?;
    /// // eigenvalues: [52 × 30] = 1560 values
    /// // eigenvectors: [52 × 30 × 30] = 46800 values
    /// ```
    pub fn execute_f64(
        device: Arc<WgpuDevice>,
        data: &[f64],
        n: usize,
        batch_size: usize,
        max_sweeps: u32,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let expected_len = batch_size * n * n;
        if data.len() != expected_len {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Expected {} elements for {} matrices of {}x{}, got {}",
                    expected_len,
                    batch_size,
                    n,
                    n,
                    data.len()
                ),
            });
        }

        let nu = n as u32;
        let batch_u = batch_size as u32;

        // Create A buffer (input matrices, will be modified in-place)
        let a_buffer = {
            let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Batched A f64"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                })
        };

        // Create V buffer (eigenvectors output)
        let v_size = (batch_size * n * n * 8) as u64;
        let v_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batched V f64"),
            size: v_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create eigenvalues buffer
        let eig_size = (batch_size * n * 8) as u64;
        let eig_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batched eigenvalues f64"),
            size: eig_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create cos/sin buffer for rotation angles [batch_size × 2]
        let cs_size = (batch_size * 2 * 8) as u64;
        let cs_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batched cs f64"),
            size: cs_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Batched Eigh f64"));

        // Create bind group layouts and pipelines

        // Init V layout (same as main params)
        let init_bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Batched Init V BGL"),
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
                ],
            });

        let init_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Batched Init V PL"),
                bind_group_layouts: &[&init_bgl],
                push_constant_ranges: &[],
            });

        let init_v_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Batched Init V"),
                    layout: Some(&init_pl),
                    module: &shader,
                    entry_point: "batched_init_V",
                cache: None,
                compilation_options: Default::default(),
                });

        let extract_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Batched Extract Eigenvalues"),
                    layout: Some(&init_pl),
                    module: &shader,
                    entry_point: "batched_extract_eigenvalues",
                cache: None,
                compilation_options: Default::default(),
                });

        // Parallel sweep layout (A, V, cs buffers)
        let sweep_bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Parallel Sweep BGL"),
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
                ],
            });

        let sweep_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Parallel Sweep PL"),
                bind_group_layouts: &[&sweep_bgl],
                push_constant_ranges: &[],
            });

        let compute_angles_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Compute Angles"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_compute_angles",
                cache: None,
                compilation_options: Default::default(),
                });

        let rotate_a_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Rotate A"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_rotate_A",
                cache: None,
                compilation_options: Default::default(),
                });

        let update_blocks_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Update Blocks"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_update_blocks",
                cache: None,
                compilation_options: Default::default(),
                });

        let rotate_v_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Rotate V"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_rotate_V",
                cache: None,
                compilation_options: Default::default(),
                });

        // Create params buffer
        let params = BatchedEighParams {
            n: nu,
            batch_size: batch_u,
            max_sweeps,
            _pad: 0,
        };
        let params_buffer = device.create_uniform_buffer("Batched Eigh Params", &params);

        // Create init bind group
        let init_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Batched Init BG"),
            layout: &init_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: v_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eig_buffer.as_entire_binding(),
                },
            ],
        });

        // Step 1: Initialize V = Identity for all matrices
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Init V Encoder"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Init V Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&init_v_pipeline);
                pass.set_bind_group(0, &init_bg, &[]);
                // Dispatch (n/16, n/16, batch_size) workgroups
                let wg_xy = nu.div_ceil(16);
                pass.dispatch_workgroups(wg_xy, wg_xy, batch_u);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Step 2: Jacobi sweeps
        // For each sweep, iterate through all (p, q) pairs
        for _sweep in 0..max_sweeps {
            for p in 0..(n - 1) {
                for q in (p + 1)..n {
                    // Create sweep params
                    let sweep_params = ParallelSweepParams {
                        n: nu,
                        batch_size: batch_u,
                        current_p: p as u32,
                        current_q: q as u32,
                    };
                    let sweep_params_buffer =
                        device.create_uniform_buffer("Sweep Params", &sweep_params);

                    // Create sweep bind group
                    let sweep_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Sweep BG"),
                        layout: &sweep_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: sweep_params_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: a_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: v_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: cs_buffer.as_entire_binding(),
                            },
                        ],
                    });

                    // 2a: Compute rotation angles for all batches
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Compute Angles"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Compute Angles Pass"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&compute_angles_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(batch_u.div_ceil(64), 1, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2b: Rotate A for all batches
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Rotate A"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Rotate A Pass"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&rotate_a_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(nu.div_ceil(64), batch_u, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2c: Update 2×2 blocks for all batches
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Update Blocks"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Update Blocks Pass"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&update_blocks_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(batch_u.div_ceil(64), 1, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2d: Rotate V for all batches
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Rotate V"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Rotate V Pass"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&rotate_v_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(nu.div_ceil(64), batch_u, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }
                }
            }
        }

        // Step 3: Extract eigenvalues from diagonal of A
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Extract Eigenvalues"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Extract Eigenvalues Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&extract_pipeline);
                pass.set_bind_group(0, &init_bg, &[]);
                pass.dispatch_workgroups(nu.div_ceil(256), batch_u, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Read back results
        let eigenvalues = Self::read_f64_buffer(&device, &eig_buffer, batch_size * n)?;
        let eigenvectors = Self::read_f64_buffer(&device, &v_buffer, batch_size * n * n)?;

        Ok((eigenvalues, eigenvectors))
    }

    /// **SINGLE-DISPATCH** batched eigenvalue decomposition — eliminates poll bottleneck
    ///
    /// This is the critical evolution for GPU-resident SCF loops. Instead of
    /// ~8000 `queue.submit()` calls per batch, this uses exactly ONE dispatch.
    ///
    /// # Performance
    ///
    /// For n=12, batch=40:
    /// - Previous: 4 × 66 × 30 = 7,920 submits (66 rotations/sweep, 30 sweeps)
    /// - This: 1 submit
    ///
    /// # Constraints
    ///
    /// - Maximum n=32 (limited by workgroup shared memory: 32×32×8×2 = 16KB)
    /// - For larger matrices, use `execute_f64()` or `execute_f64_buffers()`
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `data` - Packed matrices [batch_size × n × n] in row-major order (f64)
    /// * `n` - Matrix dimension (must be ≤ 32)
    /// * `batch_size` - Number of matrices
    /// * `max_sweeps` - Maximum Jacobi sweeps
    /// * `tolerance` - Convergence tolerance for off-diagonal elements
    ///
    /// # Returns
    /// Tuple (eigenvalues, eigenvectors) where:
    /// - eigenvalues: [batch_size × n] f64
    /// - eigenvectors: [batch_size × n × n] f64
    ///
    /// # Example (hotSpring HFB)
    /// ```ignore
    /// // 40 matrices of 12×12 dimension - ONE dispatch
    /// let (eigenvalues, eigenvectors) = BatchedEighGpu::execute_single_dispatch(
    ///     device, &packed_hamiltonians, 12, 40, 30, 1e-12
    /// )?;
    /// ```
    pub fn execute_single_dispatch(
        device: Arc<WgpuDevice>,
        data: &[f64],
        n: usize,
        batch_size: usize,
        max_sweeps: u32,
        tolerance: f64,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        const MAX_N: usize = 32;

        if n > MAX_N {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Single-dispatch eigensolve limited to n≤{}, got n={}. Use execute_f64() for larger matrices.",
                    MAX_N, n
                ),
            });
        }

        if data.len() != batch_size * n * n {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Data length {} does not match batch_size={} × n²={}",
                    data.len(),
                    batch_size,
                    n * n
                ),
            });
        }

        // Create GPU buffers
        let a_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SingleDispatch A"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let v_size = (batch_size * n * n * 8) as u64;
        let v_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SingleDispatch V"),
            size: v_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let eig_size = (batch_size * n * 8) as u64;
        let eig_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SingleDispatch Eigenvalues"),
            size: eig_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Params buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SingleDispatchParams {
            n: u32,
            batch_size: u32,
            max_sweeps: u32,
            tolerance: f32,
        }

        let params = SingleDispatchParams {
            n: n as u32,
            batch_size: batch_size as u32,
            max_sweeps,
            tolerance: tolerance as f32,
        };
        let params_buffer = device.create_uniform_buffer("SingleDispatch Params", &params);

        // Compile shader
        let shader = device.compile_shader(
            Self::single_dispatch_shader(),
            Some("Batched Eigh Single-Dispatch f64"),
        );

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SingleDispatch BGL"),
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
                ],
            });

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SingleDispatch PL"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("SingleDispatch Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "batched_eigh_single_dispatch",
            cache: None,
            compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SingleDispatch BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: v_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eig_buffer.as_entire_binding(),
                },
            ],
        });

        // *** THE KEY: SINGLE DISPATCH ***
        // One workgroup per matrix in batch
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SingleDispatch Encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SingleDispatch Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // batch_size workgroups, each processes one matrix
            pass.dispatch_workgroups(batch_size as u32, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let eigenvalues = Self::read_f64_buffer(&device, &eig_buffer, batch_size * n)?;
        let eigenvectors = Self::read_f64_buffer(&device, &v_buffer, batch_size * n * n)?;

        Ok((eigenvalues, eigenvectors))
    }

    /// **SINGLE-DISPATCH** buffer-based eigensolve — no CPU readback
    ///
    /// Combines single-dispatch efficiency with buffer-based API for GPU-resident loops.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `matrices_buffer` - Input buffer [batch_size × n × n] f64
    /// * `eigenvalues_buffer` - Output buffer [batch_size × n] f64
    /// * `eigenvectors_buffer` - Output buffer [batch_size × n × n] f64
    /// * `n` - Matrix dimension (must be ≤ 32)
    /// * `batch_size` - Number of matrices
    /// * `max_sweeps` - Maximum Jacobi sweeps
    /// * `tolerance` - Convergence tolerance
    ///
    /// # Returns
    /// `Ok(())` - results in output buffers, no CPU copy
    pub fn execute_single_dispatch_buffers(
        device: &Arc<WgpuDevice>,
        matrices_buffer: &wgpu::Buffer,
        eigenvalues_buffer: &wgpu::Buffer,
        eigenvectors_buffer: &wgpu::Buffer,
        n: usize,
        batch_size: usize,
        max_sweeps: u32,
        tolerance: f64,
    ) -> Result<()> {
        const MAX_N: usize = 32;

        if n > MAX_N {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Single-dispatch eigensolve limited to n≤{}, got n={}",
                    MAX_N, n
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SingleDispatchParams {
            n: u32,
            batch_size: u32,
            max_sweeps: u32,
            tolerance: f32,
        }

        let params = SingleDispatchParams {
            n: n as u32,
            batch_size: batch_size as u32,
            max_sweeps,
            tolerance: tolerance as f32,
        };
        let params_buffer =
            device.create_uniform_buffer("SingleDispatch Params (buffers)", &params);

        let shader = device.compile_shader(
            Self::single_dispatch_shader(),
            Some("Batched Eigh Single-Dispatch f64 (buffers)"),
        );

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SingleDispatch BGL (buffers)"),
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
                ],
            });

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SingleDispatch PL (buffers)"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("SingleDispatch Pipeline (buffers)"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "batched_eigh_single_dispatch",
            cache: None,
            compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SingleDispatch BG (buffers)"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: matrices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: eigenvectors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eigenvalues_buffer.as_entire_binding(),
                },
            ],
        });

        // Single dispatch - one workgroup per matrix
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SingleDispatch Encoder (buffers)"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SingleDispatch Pass (buffers)"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(batch_size as u32, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    /// Convenience method for processing a batch of matrices
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `matrices` - Vector of symmetric matrices, each as flattened [n×n] row-major f64
    /// * `n` - Matrix dimension
    ///
    /// # Returns
    /// Vector of (eigenvalues, eigenvectors) tuples, one per input matrix
    pub fn execute_batch(
        device: Arc<WgpuDevice>,
        matrices: &[Vec<f64>],
        n: usize,
    ) -> Result<Vec<(Vec<f64>, Vec<f64>)>> {
        if matrices.is_empty() {
            return Ok(vec![]);
        }

        // Validate all matrices have correct size
        for (i, m) in matrices.iter().enumerate() {
            if m.len() != n * n {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "Matrix {} has {} elements, expected {} for {}x{} matrix",
                        i,
                        m.len(),
                        n * n,
                        n,
                        n
                    ),
                });
            }
        }

        let batch_size = matrices.len();

        // Pack matrices into single buffer
        let packed: Vec<f64> = matrices.iter().flat_map(|m| m.iter().copied()).collect();

        // Execute batched eigendecomposition
        let (eigenvalues_flat, eigenvectors_flat) =
            Self::execute_f64(device, &packed, n, batch_size, 30)?;

        // Unpack results
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let eig_start = i * n;
            let eig_end = eig_start + n;
            let vec_start = i * n * n;
            let vec_end = vec_start + n * n;

            results.push((
                eigenvalues_flat[eig_start..eig_end].to_vec(),
                eigenvectors_flat[vec_start..vec_end].to_vec(),
            ));
        }

        Ok(results)
    }

    /// Create GPU buffers for GPU-resident eigenvalue decomposition
    ///
    /// This is the first step for using `execute_f64_buffers()`. The returned buffers
    /// can be reused across multiple solver iterations without CPU↔GPU transfers.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `n` - Matrix dimension
    /// * `batch_size` - Number of matrices
    ///
    /// # Returns
    /// Tuple of (matrices_buffer, eigenvalues_buffer, eigenvectors_buffer)
    /// - matrices_buffer: [batch_size × n × n] f64 - input/working buffer
    /// - eigenvalues_buffer: [batch_size × n] f64 - output eigenvalues
    /// - eigenvectors_buffer: [batch_size × n × n] f64 - output eigenvectors
    ///
    /// # Example
    /// ```ignore
    /// let (matrices_buf, eigenvalues_buf, eigenvectors_buf) =
    ///     BatchedEighGpu::create_buffers(&device, n, batch_size)?;
    /// // Write initial matrices to matrices_buf (from another GPU operation)
    /// // Then execute without CPU readback:
    /// BatchedEighGpu::execute_f64_buffers(&device, &matrices_buf, &eigenvalues_buf, &eigenvectors_buf, n, batch_size, 30)?;
    /// // eigenvalues_buf and eigenvectors_buf now contain results (still on GPU)
    /// ```
    pub fn create_buffers(
        device: &Arc<WgpuDevice>,
        n: usize,
        batch_size: usize,
    ) -> Result<(wgpu::Buffer, wgpu::Buffer, wgpu::Buffer)> {
        let nu = n as u32;
        let batch_u = batch_size as u32;

        // Matrices buffer (input, modified in-place during Jacobi)
        let matrices_size = (batch_size * n * n * 8) as u64;
        let matrices_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedEigh matrices f64"),
            size: matrices_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Eigenvalues buffer (output)
        let eigenvalues_size = (batch_size * n * 8) as u64;
        let eigenvalues_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedEigh eigenvalues f64"),
            size: eigenvalues_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Eigenvectors buffer (output)
        let eigenvectors_size = (batch_size * n * n * 8) as u64;
        let eigenvectors_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedEigh eigenvectors f64"),
            size: eigenvectors_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let _ = (nu, batch_u); // Suppress unused warnings
        Ok((matrices_buffer, eigenvalues_buffer, eigenvectors_buffer))
    }

    /// Execute batched eigenvalue decomposition on GPU **without CPU readback**
    ///
    /// This is the GPU-resident variant for use in pipelines where data should
    /// stay on GPU between operations (e.g., SCF iteration loops).
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `matrices_buffer` - Input buffer [batch_size × n × n] f64 (modified in-place)
    /// * `eigenvalues_buffer` - Output buffer [batch_size × n] f64
    /// * `eigenvectors_buffer` - Output buffer [batch_size × n × n] f64
    /// * `n` - Matrix dimension
    /// * `batch_size` - Number of matrices
    /// * `max_sweeps` - Maximum Jacobi sweeps
    ///
    /// # Returns
    /// `Ok(())` on success - results are in the output buffers (no CPU copy)
    ///
    /// # Example (GPU-resident SCF loop)
    /// ```ignore
    /// // Pre-allocate buffers once
    /// let (h_buf, eig_buf, vec_buf) = BatchedEighGpu::create_buffers(&device, n, batch)?;
    ///
    /// for iteration in 0..max_iter {
    ///     // Previous stage writes Hamiltonian to h_buf (GPU→GPU)
    ///     hamiltonian_builder.execute_to_buffer(&h_buf)?;
    ///
    ///     // Eigensolve without CPU readback
    ///     BatchedEighGpu::execute_f64_buffers(
    ///         &device, &h_buf, &eig_buf, &vec_buf, n, batch, 30
    ///     )?;
    ///
    ///     // Next stage reads from eig_buf/vec_buf (GPU→GPU)
    ///     bcs_pairing.execute_from_buffers(&eig_buf, &occupations_buf)?;
    ///
    ///     // Only read back for convergence check (single scalar)
    ///     let converged = convergence_check(&device, &energy_buf)?;
    ///     if converged { break; }
    /// }
    /// ```
    pub fn execute_f64_buffers(
        device: &Arc<WgpuDevice>,
        matrices_buffer: &wgpu::Buffer,
        eigenvalues_buffer: &wgpu::Buffer,
        eigenvectors_buffer: &wgpu::Buffer,
        n: usize,
        batch_size: usize,
        max_sweeps: u32,
    ) -> Result<()> {
        let nu = n as u32;
        let batch_u = batch_size as u32;

        // Create working buffer for cos/sin rotation angles
        let cs_size = (batch_size * 2 * 8) as u64;
        let cs_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedEigh cs f64"),
            size: cs_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Batched Eigh f64 (buffers)"));

        // Create bind group layouts and pipelines (same as execute_f64)

        // Init V layout
        let init_bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Batched Init V BGL (buffers)"),
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
                ],
            });

        let init_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Batched Init V PL (buffers)"),
                bind_group_layouts: &[&init_bgl],
                push_constant_ranges: &[],
            });

        let init_v_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Batched Init V (buffers)"),
                    layout: Some(&init_pl),
                    module: &shader,
                    entry_point: "batched_init_V",
                cache: None,
                compilation_options: Default::default(),
                });

        let extract_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Batched Extract Eigenvalues (buffers)"),
                    layout: Some(&init_pl),
                    module: &shader,
                    entry_point: "batched_extract_eigenvalues",
                cache: None,
                compilation_options: Default::default(),
                });

        // Parallel sweep layout
        let sweep_bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Parallel Sweep BGL (buffers)"),
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
                ],
            });

        let sweep_pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Parallel Sweep PL (buffers)"),
                bind_group_layouts: &[&sweep_bgl],
                push_constant_ranges: &[],
            });

        let compute_angles_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Compute Angles (buffers)"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_compute_angles",
                cache: None,
                compilation_options: Default::default(),
                });

        let rotate_a_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Rotate A (buffers)"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_rotate_A",
                cache: None,
                compilation_options: Default::default(),
                });

        let update_blocks_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Update Blocks (buffers)"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_update_blocks",
                cache: None,
                compilation_options: Default::default(),
                });

        let rotate_v_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Parallel Rotate V (buffers)"),
                    layout: Some(&sweep_pl),
                    module: &shader,
                    entry_point: "parallel_rotate_V",
                cache: None,
                compilation_options: Default::default(),
                });

        // Create params buffer
        let params = BatchedEighParams {
            n: nu,
            batch_size: batch_u,
            max_sweeps,
            _pad: 0,
        };
        let params_buffer = device.create_uniform_buffer("Batched Eigh Params (buffers)", &params);

        // Create init bind group using provided buffers
        let init_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Batched Init BG (buffers)"),
            layout: &init_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: matrices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: eigenvectors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eigenvalues_buffer.as_entire_binding(),
                },
            ],
        });

        // Step 1: Initialize V = Identity for all matrices
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Init V Encoder (buffers)"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Init V Pass (buffers)"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&init_v_pipeline);
                pass.set_bind_group(0, &init_bg, &[]);
                let wg_xy = nu.div_ceil(16);
                pass.dispatch_workgroups(wg_xy, wg_xy, batch_u);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Step 2: Jacobi sweeps
        for _sweep in 0..max_sweeps {
            for p in 0..(n - 1) {
                for q in (p + 1)..n {
                    let sweep_params = ParallelSweepParams {
                        n: nu,
                        batch_size: batch_u,
                        current_p: p as u32,
                        current_q: q as u32,
                    };
                    let sweep_params_buffer =
                        device.create_uniform_buffer("Sweep Params (buffers)", &sweep_params);

                    let sweep_bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Sweep BG (buffers)"),
                        layout: &sweep_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: sweep_params_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: matrices_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: eigenvectors_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: cs_buffer.as_entire_binding(),
                            },
                        ],
                    });

                    // 2a: Compute rotation angles
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Compute Angles (buffers)"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Compute Angles Pass (buffers)"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&compute_angles_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(batch_u.div_ceil(64), 1, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2b: Rotate A
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Rotate A (buffers)"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Rotate A Pass (buffers)"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&rotate_a_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(nu.div_ceil(64), batch_u, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2c: Update 2×2 blocks
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Update Blocks (buffers)"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Update Blocks Pass (buffers)"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&update_blocks_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(batch_u.div_ceil(64), 1, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }

                    // 2d: Rotate V
                    {
                        let mut encoder =
                            device
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Rotate V (buffers)"),
                                });
                        {
                            let mut pass =
                                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                    label: Some("Rotate V Pass (buffers)"),
                                    timestamp_writes: None,
                                });
                            pass.set_pipeline(&rotate_v_pipeline);
                            pass.set_bind_group(0, &sweep_bg, &[]);
                            pass.dispatch_workgroups(nu.div_ceil(64), batch_u, 1);
                        }
                        device.queue.submit(Some(encoder.finish()));
                    }
                }
            }
        }

        // Step 3: Extract eigenvalues from diagonal
        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Extract Eigenvalues (buffers)"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Extract Eigenvalues Pass (buffers)"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&extract_pipeline);
                pass.set_bind_group(0, &init_bg, &[]);
                pass.dispatch_workgroups(nu.div_ceil(256), batch_u, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // NO CPU READBACK - results stay in eigenvalues_buffer and eigenvectors_buffer
        Ok(())
    }

    /// Read eigenvalues from a GPU buffer to CPU (for convergence checks)
    ///
    /// This is the minimal CPU readback needed during SCF iteration - just the
    /// eigenvalues for energy/convergence calculations. The eigenvectors stay on GPU.
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `eigenvalues_buffer` - Buffer containing eigenvalues [batch_size × n] f64
    /// * `n` - Matrix dimension
    /// * `batch_size` - Number of matrices
    ///
    /// # Returns
    /// `Vec<f64>` of eigenvalues (flattened)
    pub fn read_eigenvalues(
        device: &Arc<WgpuDevice>,
        eigenvalues_buffer: &wgpu::Buffer,
        n: usize,
        batch_size: usize,
    ) -> Result<Vec<f64>> {
        Self::read_f64_buffer(device, eigenvalues_buffer, batch_size * n)
    }

    /// Read eigenvectors from a GPU buffer to CPU (optional, only when needed)
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `eigenvectors_buffer` - Buffer containing eigenvectors [batch_size × n × n] f64
    /// * `n` - Matrix dimension
    /// * `batch_size` - Number of matrices
    ///
    /// # Returns
    /// `Vec<f64>` of eigenvectors (flattened row-major)
    pub fn read_eigenvectors(
        device: &Arc<WgpuDevice>,
        eigenvectors_buffer: &wgpu::Buffer,
        n: usize,
        batch_size: usize,
    ) -> Result<Vec<f64>> {
        Self::read_f64_buffer(device, eigenvectors_buffer, batch_size * n * n)
    }

    /// Helper: Read f64 buffer from GPU
    fn read_f64_buffer(
        device: &Arc<WgpuDevice>,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        let staging = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("f64 staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("f64 readback"),
            });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 8) as u64);
        device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(
            wgpu::MapMode::Read,
            move |result: std::result::Result<(), wgpu::BufferAsyncError>| {
                let _ = sender.send(result);
            },
        );
        device.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| {
                f64::from_le_bytes(
                    chunk
                        .try_into()
                        .expect("chunks_exact(8) yields 8-byte chunks"),
                )
            })
            .collect();
        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq_f64(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[tokio::test]
    async fn test_batched_eigh_single_2x2() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // Single 2×2 symmetric matrix: [[4, 2], [2, 3]]
        // Eigenvalues: 5, 2 (trace=7, det=8)
        let data = vec![4.0_f64, 2.0, 2.0, 3.0];
        let (eigenvalues, eigenvectors) =
            BatchedEighGpu::execute_f64(device.clone(), &data, 2, 1, 30).unwrap();

        assert_eq!(eigenvalues.len(), 2);
        assert_eq!(eigenvectors.len(), 4);

        // Check trace (sum of eigenvalues = 7)
        let trace = eigenvalues[0] + eigenvalues[1];
        assert!(
            approx_eq_f64(trace, 7.0, 1e-6),
            "Trace should be 7, got {}",
            trace
        );

        // Check product (det = 8)
        let det = eigenvalues[0] * eigenvalues[1];
        assert!(
            approx_eq_f64(det, 8.0, 1e-4),
            "Determinant should be 8, got {}",
            det
        );
    }

    #[tokio::test]
    async fn test_batched_eigh_identity_batch() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // Batch of 3 identity matrices (2×2)
        let data = vec![
            1.0_f64, 0.0, 0.0, 1.0, // I_1
            1.0, 0.0, 0.0, 1.0, // I_2
            1.0, 0.0, 0.0, 1.0, // I_3
        ];
        let (eigenvalues, eigenvectors) =
            BatchedEighGpu::execute_f64(device.clone(), &data, 2, 3, 10).unwrap();

        assert_eq!(eigenvalues.len(), 6); // 3 × 2
        assert_eq!(eigenvectors.len(), 12); // 3 × 4

        // All eigenvalues should be 1
        for (i, &val) in eigenvalues.iter().enumerate() {
            assert!(
                approx_eq_f64(val, 1.0, 1e-6),
                "Eigenvalue {} should be 1, got {}",
                i,
                val
            );
        }
    }

    #[tokio::test]
    async fn test_batched_eigh_hfb_scale() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // Simulate HFB scale: 52 matrices of 20×20 dimension
        // Use diagonal matrices for easy verification
        let n = 20;
        let batch_size = 52;

        // Create batch of diagonal matrices with known eigenvalues
        let mut data = vec![0.0_f64; batch_size * n * n];
        for b in 0..batch_size {
            for i in 0..n {
                // Eigenvalues: 1, 2, 3, ..., n for each matrix
                data[b * n * n + i * n + i] = (i + 1) as f64;
            }
        }

        let (eigenvalues, _eigenvectors) =
            BatchedEighGpu::execute_f64(device.clone(), &data, n, batch_size, 30).unwrap();

        assert_eq!(eigenvalues.len(), batch_size * n);

        // Check first matrix eigenvalues sum to 1+2+...+20 = 210
        let first_sum: f64 = eigenvalues[0..n].iter().sum();
        assert!(
            approx_eq_f64(first_sum, 210.0, 1e-3),
            "First matrix eigenvalue sum should be 210, got {}",
            first_sum
        );

        // Check last matrix too
        let last_start = (batch_size - 1) * n;
        let last_sum: f64 = eigenvalues[last_start..last_start + n].iter().sum();
        assert!(
            approx_eq_f64(last_sum, 210.0, 1e-3),
            "Last matrix eigenvalue sum should be 210, got {}",
            last_sum
        );
    }

    #[tokio::test]
    async fn test_batched_eigh_execute_batch() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // Test the convenience batch method
        let matrices = vec![
            vec![4.0_f64, 2.0, 2.0, 3.0], // Eigenvalues ~5, 2
            vec![1.0_f64, 0.0, 0.0, 1.0], // Eigenvalues 1, 1
            vec![2.0_f64, 1.0, 1.0, 2.0], // Eigenvalues 3, 1
        ];

        let results = BatchedEighGpu::execute_batch(device, &matrices, 2).unwrap();

        assert_eq!(results.len(), 3);

        // Verify traces
        let trace_0: f64 = results[0].0.iter().sum();
        let trace_1: f64 = results[1].0.iter().sum();
        let trace_2: f64 = results[2].0.iter().sum();

        assert!(
            approx_eq_f64(trace_0, 7.0, 1e-4),
            "Matrix 0 trace should be 7"
        );
        assert!(
            approx_eq_f64(trace_1, 2.0, 1e-4),
            "Matrix 1 trace should be 2"
        );
        assert!(
            approx_eq_f64(trace_2, 4.0, 1e-4),
            "Matrix 2 trace should be 4"
        );
    }

    #[tokio::test]
    async fn test_batched_eigh_buffer_api() {
        // Test the GPU-resident buffer-based API
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 2;
        let batch_size = 2;

        // Create persistent buffers (as would be done in SCF loop setup)
        let (matrices_buf, eigenvalues_buf, eigenvectors_buf) =
            BatchedEighGpu::create_buffers(&device, n, batch_size).unwrap();

        // Prepare input data: two 2x2 symmetric matrices
        // Matrix 0: [[4, 2], [2, 3]] -> eigenvalues ~5, 2 (trace=7)
        // Matrix 1: [[2, 1], [1, 2]] -> eigenvalues  3, 1 (trace=3)
        let input_data: Vec<f64> = vec![
            4.0, 2.0, 2.0, 3.0, // Matrix 0
            2.0, 1.0, 1.0, 2.0, // Matrix 1
        ];

        // Upload input to GPU buffer (simulates Hamiltonian builder output)
        device
            .queue
            .write_buffer(&matrices_buf, 0, bytemuck::cast_slice(&input_data));

        // Execute eigensolve WITHOUT CPU readback
        BatchedEighGpu::execute_f64_buffers(
            &device,
            &matrices_buf,
            &eigenvalues_buf,
            &eigenvectors_buf,
            n,
            batch_size,
            30,
        )
        .unwrap();

        // Now read back eigenvalues only (minimal readback for convergence check)
        let eigenvalues =
            BatchedEighGpu::read_eigenvalues(&device, &eigenvalues_buf, n, batch_size).unwrap();

        assert_eq!(eigenvalues.len(), 4); // 2 matrices × 2 eigenvalues each

        // Verify Matrix 0 trace = 7
        let trace_0 = eigenvalues[0] + eigenvalues[1];
        assert!(
            approx_eq_f64(trace_0, 7.0, 1e-4),
            "Matrix 0 trace should be 7, got {}",
            trace_0
        );

        // Verify Matrix 1 trace = 4
        let trace_1 = eigenvalues[2] + eigenvalues[3];
        assert!(
            approx_eq_f64(trace_1, 4.0, 1e-4),
            "Matrix 1 trace should be 4, got {}",
            trace_1
        );

        // Optionally verify eigenvectors are available
        let eigenvectors =
            BatchedEighGpu::read_eigenvectors(&device, &eigenvectors_buf, n, batch_size).unwrap();
        assert_eq!(eigenvectors.len(), 8); // 2 matrices × 2×2 eigenvectors each
    }

    #[tokio::test]
    async fn test_batched_eigh_buffer_reuse() {
        // Test that buffers can be reused across iterations (GPU-resident SCF pattern)
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 2;
        let batch_size = 1;

        // Create buffers once (as in SCF setup)
        let (matrices_buf, eigenvalues_buf, eigenvectors_buf) =
            BatchedEighGpu::create_buffers(&device, n, batch_size).unwrap();

        // Iteration 1: Identity matrix -> eigenvalues (1, 1)
        let data_1: Vec<f64> = vec![1.0, 0.0, 0.0, 1.0];
        device
            .queue
            .write_buffer(&matrices_buf, 0, bytemuck::cast_slice(&data_1));
        BatchedEighGpu::execute_f64_buffers(
            &device,
            &matrices_buf,
            &eigenvalues_buf,
            &eigenvectors_buf,
            n,
            batch_size,
            10,
        )
        .unwrap();
        let eig_1 =
            BatchedEighGpu::read_eigenvalues(&device, &eigenvalues_buf, n, batch_size).unwrap();
        let trace_1: f64 = eig_1.iter().sum();
        assert!(
            approx_eq_f64(trace_1, 2.0, 1e-6),
            "Iteration 1 trace should be 2"
        );

        // Iteration 2: New matrix [[3, 1], [1, 3]] -> eigenvalues (4, 2), trace=6
        let data_2: Vec<f64> = vec![3.0, 1.0, 1.0, 3.0];
        device
            .queue
            .write_buffer(&matrices_buf, 0, bytemuck::cast_slice(&data_2));
        BatchedEighGpu::execute_f64_buffers(
            &device,
            &matrices_buf,
            &eigenvalues_buf,
            &eigenvectors_buf,
            n,
            batch_size,
            10,
        )
        .unwrap();
        let eig_2 =
            BatchedEighGpu::read_eigenvalues(&device, &eigenvalues_buf, n, batch_size).unwrap();
        let trace_2: f64 = eig_2.iter().sum();
        assert!(
            approx_eq_f64(trace_2, 6.0, 1e-4),
            "Iteration 2 trace should be 6"
        );

        // Buffers successfully reused across iterations
    }

    #[tokio::test]
    async fn test_single_dispatch_basic() {
        // Test the single-dispatch eigensolve (TIER 1.1 critical feature)
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // Two 2×2 matrices
        let data: Vec<f64> = vec![
            4.0, 2.0, 2.0, 3.0, // Matrix 0: trace=7
            2.0, 1.0, 1.0, 2.0, // Matrix 1: trace=4
        ];

        let (eigenvalues, eigenvectors) =
            BatchedEighGpu::execute_single_dispatch(device.clone(), &data, 2, 2, 30, 1e-12)
                .unwrap();

        assert_eq!(eigenvalues.len(), 4);
        assert_eq!(eigenvectors.len(), 8);

        // Verify traces
        let trace_0 = eigenvalues[0] + eigenvalues[1];
        let trace_1 = eigenvalues[2] + eigenvalues[3];

        assert!(
            approx_eq_f64(trace_0, 7.0, 1e-4),
            "Matrix 0 trace should be 7, got {}",
            trace_0
        );
        assert!(
            approx_eq_f64(trace_1, 4.0, 1e-4),
            "Matrix 1 trace should be 4, got {}",
            trace_1
        );
    }

    #[tokio::test]
    async fn test_single_dispatch_hotspring_scale() {
        // Test at hotSpring scale: 40 matrices of 12×12
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 12;
        let batch_size = 40;

        // Create batch of diagonal matrices with known eigenvalues
        let mut data = vec![0.0_f64; batch_size * n * n];
        for b in 0..batch_size {
            for i in 0..n {
                // Eigenvalues: 1, 2, 3, ..., n for each matrix
                data[b * n * n + i * n + i] = (i + 1) as f64;
            }
        }

        // Single dispatch for all 40 matrices
        let (eigenvalues, _eigenvectors) = BatchedEighGpu::execute_single_dispatch(
            device.clone(),
            &data,
            n,
            batch_size,
            30,
            1e-12,
        )
        .unwrap();

        assert_eq!(eigenvalues.len(), batch_size * n);

        // Check first matrix: eigenvalue sum = 1+2+...+12 = 78
        let first_sum: f64 = eigenvalues[0..n].iter().sum();
        assert!(
            approx_eq_f64(first_sum, 78.0, 1e-3),
            "First matrix eigenvalue sum should be 78, got {}",
            first_sum
        );

        // Check last matrix
        let last_start = (batch_size - 1) * n;
        let last_sum: f64 = eigenvalues[last_start..last_start + n].iter().sum();
        assert!(
            approx_eq_f64(last_sum, 78.0, 1e-3),
            "Last matrix eigenvalue sum should be 78, got {}",
            last_sum
        );
    }

    #[tokio::test]
    async fn test_single_dispatch_buffers() {
        // Test single-dispatch buffer API (GPU-resident, no CPU readback)
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let n = 4;
        let batch_size = 3;

        // Create buffers
        let (matrices_buf, eigenvalues_buf, eigenvectors_buf) =
            BatchedEighGpu::create_buffers(&device, n, batch_size).unwrap();

        // Prepare diagonal matrices with known eigenvalues
        let mut data = vec![0.0_f64; batch_size * n * n];
        for b in 0..batch_size {
            for i in 0..n {
                data[b * n * n + i * n + i] = ((b + 1) * (i + 1)) as f64;
            }
        }

        device
            .queue
            .write_buffer(&matrices_buf, 0, bytemuck::cast_slice(&data));

        // Execute single-dispatch without CPU readback
        BatchedEighGpu::execute_single_dispatch_buffers(
            &device,
            &matrices_buf,
            &eigenvalues_buf,
            &eigenvectors_buf,
            n,
            batch_size,
            30,
            1e-12,
        )
        .unwrap();

        // Read back eigenvalues
        let eigenvalues =
            BatchedEighGpu::read_eigenvalues(&device, &eigenvalues_buf, n, batch_size).unwrap();

        // Matrix 0: eigenvalues 1,2,3,4 -> sum=10
        let sum_0: f64 = eigenvalues[0..n].iter().sum();
        assert!(
            approx_eq_f64(sum_0, 10.0, 1e-4),
            "Matrix 0 sum should be 10, got {}",
            sum_0
        );

        // Matrix 1: eigenvalues 2,4,6,8 -> sum=20
        let sum_1: f64 = eigenvalues[n..2 * n].iter().sum();
        assert!(
            approx_eq_f64(sum_1, 20.0, 1e-4),
            "Matrix 1 sum should be 20, got {}",
            sum_1
        );
    }
}
