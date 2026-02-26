//! Fused Map-Reduce at f64 precision — single-dispatch unified pattern
//!
//! UNIFIED PATTERN (Feb 16 2026) — Serves all springs:
//! - wetSpring: Shannon entropy, Simpson index
//! - airSpring: ET₀ sums, water balance totals
//! - hotSpring: Convergence norms, energy functionals
//!
//! # Architecture
//!
//! Single GPU dispatch combines map + reduce:
//! 1. Each thread maps multiple elements (grid-stride loop)
//! 2. Workgroup tree reduction in shared memory
//! 3. Thread 0 writes partial to global memory
//! 4. (Optional) Second pass reduces partials to final scalar
//!
//! # Performance
//!
//! - Eliminates intermediate buffer between map and reduce
//! - Single dispatch vs 2 dispatches for separate map + reduce
//! - Memory bandwidth: 1 read pass vs 2 read + 1 write
//!
//! # Example
//!
//! ```rust,ignore
//! use barracuda::ops::fused_map_reduce_f64::{FusedMapReduceF64, MapOp, ReduceOp};
//!
//! let fmr = FusedMapReduceF64::new(device.clone())?;
//!
//! // Shannon entropy: -Σ p * log(p)
//! let counts = vec![10.0, 20.0, 30.0, 40.0];
//! let total: f64 = counts.iter().sum();
//! let shannon = fmr.execute(&counts, total, MapOp::Shannon, ReduceOp::Sum)?;
//!
//! // Simpson index: Σ p²
//! let simpson = fmr.execute(&counts, total, MapOp::Simpson, ReduceOp::Sum)?;
//! ```

use crate::device::WgpuDevice;
use crate::error::Result;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

const FMR_WORKGROUP_SIZE: usize = 256;
const FMR_MAX_SINGLE_PASS_WORKGROUPS: usize = 256;

/// Map operations available for fused map-reduce
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MapOp {
    /// Identity: f(x) = x
    Identity = 0,
    /// Shannon contribution: f(x) = -p * log(p) where p = x / total
    Shannon = 1,
    /// Simpson contribution: f(x) = p² where p = x / total
    Simpson = 2,
    /// Square: f(x) = x²
    Square = 3,
    /// Absolute value: f(x) = |x|
    Abs = 4,
    /// Natural log: f(x) = log(x)
    Log = 5,
    /// Negate: f(x) = -x
    Negate = 6,
}

/// Reduce operations available for fused map-reduce
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ReduceOp {
    /// Sum: a + b
    Sum = 0,
    /// Maximum: max(a, b)
    Max = 1,
    /// Minimum: min(a, b)
    Min = 2,
    /// Product (log-domain): log(a) + log(b) → exp to get product
    Product = 3,
}

/// Parameters for fused map-reduce shader
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    n: u32,
    n_workgroups: u32,
    total: f64,
    map_op: u32,
    reduce_op: u32,
}

/// Fused Map-Reduce executor for f64 data
///
/// Combines element-wise map operation with parallel reduction in a single
/// GPU dispatch, minimizing memory bandwidth and dispatch overhead.
pub struct FusedMapReduceF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    pipeline_partials: wgpu::ComputePipeline,
}

impl FusedMapReduceF64 {
    /// Create a new fused map-reduce executor
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let shader_source = include_str!("../shaders/reduce/fused_map_reduce_f64.wgsl");
        let shader_module =
            device.compile_shader_f64(shader_source, Some("FusedMapReduceF64 Shader"));

        // Pipeline for main map-reduce pass
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FusedMapReduceF64 Pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "fused_map_reduce",
                cache: None,
                compilation_options: Default::default(),
            });

        // Pipeline for reducing partials
        let pipeline_partials =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("FusedMapReduceF64 Partials Pipeline"),
                    layout: None,
                    module: &shader_module,
                    entry_point: "reduce_partials",
                    cache: None,
                    compilation_options: Default::default(),
                });

        Ok(Self {
            device,
            pipeline,
            pipeline_partials,
        })
    }

    /// Execute fused map-reduce on input data
    ///
    /// # Arguments
    /// * `data` - Input array
    /// * `total` - Normalization constant (used by Shannon, Simpson)
    /// * `map_op` - Element-wise transformation to apply
    /// * `reduce_op` - Reduction operation to combine results
    ///
    /// # Returns
    /// The reduced scalar result
    pub fn execute(
        &self,
        data: &[f64],
        total: f64,
        map_op: MapOp,
        reduce_op: ReduceOp,
    ) -> Result<f64> {
        let n = data.len();
        if n == 0 {
            return Ok(match reduce_op {
                ReduceOp::Sum => 0.0,
                ReduceOp::Max => f64::NEG_INFINITY,
                ReduceOp::Min => f64::INFINITY,
                ReduceOp::Product => 0.0, // log(1) = 0
            });
        }

        // Create input buffer
        let input_buffer =
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("FMR Input"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let workgroup_size = FMR_WORKGROUP_SIZE;
        let n_workgroups = n.div_ceil(workgroup_size);

        // Create output buffer for partials
        let output_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FMR Output"),
            size: (n_workgroups * 8) as u64, // f64 = 8 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create params buffer
        let params = Params {
            n: n as u32,
            n_workgroups: n_workgroups as u32,
            total,
            map_op: map_op as u32,
            reduce_op: reduce_op as u32,
        };
        let params_buffer =
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("FMR Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Create bind group for pass 1
        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FMR Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buffer.as_entire_binding(),
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

        // Single command encoder for all passes — ensures proper GPU synchronization
        // between the map-reduce pass and the partials reduction pass. Using separate
        // submissions caused buffer conflicts on some drivers (TS-004).
        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("FMR Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FMR Pass 1"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(n_workgroups as u32, 1, 1);
        }

        if n_workgroups > 1 && n_workgroups <= FMR_MAX_SINGLE_PASS_WORKGROUPS {
            // TS-004: Use separate partials_buffer for pass 2 input to avoid buffer conflict.
            // Some drivers reject using output_buffer as both pass 1 write and pass 2 read.
            let partials_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("FMR Partials"),
                size: (n_workgroups * 8) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let final_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("FMR Partials Output"),
                size: 8,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Copy pass 1 output to partials_buffer so pass 2 reads from a distinct buffer
            encoder.copy_buffer_to_buffer(
                &output_buffer,
                0,
                &partials_buffer,
                0,
                (n_workgroups * 8) as u64,
            );

            let pass2_params = Params {
                n: n_workgroups as u32,
                n_workgroups: 1,
                total: 1.0,
                map_op: MapOp::Identity as u32,
                reduce_op: reduce_op as u32,
            };
            let pass2_params_buffer =
                self.device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("FMR Params Pass2"),
                        contents: bytemuck::bytes_of(&pass2_params),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

            let pass2_layout = self.pipeline_partials.get_bind_group_layout(0);
            let pass2_bind_group =
                self.device
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("FMR Bind Group Pass2"),
                        layout: &pass2_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: partials_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: final_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: pass2_params_buffer.as_entire_binding(),
                            },
                        ],
                    });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("FMR Pass 2"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline_partials);
                pass.set_bind_group(0, &pass2_bind_group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            self.device.queue.submit(Some(encoder.finish()));
            return self.read_result(&final_buffer);
        }

        if n_workgroups > FMR_MAX_SINGLE_PASS_WORKGROUPS {
            // Submit pass 1, then fall back to CPU-side reduction of partials
            self.device.queue.submit(Some(encoder.finish()));
            return self.reduce_partials_recursive(&output_buffer, n_workgroups, reduce_op);
        }

        // Single workgroup — just submit and read
        self.device.queue.submit(Some(encoder.finish()));
        self.read_result(&output_buffer)
    }

    /// Recursive reduction for very large inputs (>FMR_MAX_SINGLE_PASS_WORKGROUPS)
    fn reduce_partials_recursive(
        &self,
        buffer: &wgpu::Buffer,
        n_partials: usize,
        reduce_op: ReduceOp,
    ) -> Result<f64> {
        // For very large inputs, read partials to CPU and finish there
        // This is rare (>65K elements) and CPU finish is fast
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FMR Staging"),
            size: (n_partials * 8) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("FMR Copy Encoder"),
                });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (n_partials * 8) as u64);
        self.device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let partials: Vec<f64> = data
            .chunks_exact(8)
            .map(|b| f64::from_le_bytes(b.try_into().expect("chunks_exact(8) guarantees 8 bytes")))
            .collect();
        drop(data);
        staging.unmap();

        // Finish on CPU
        let result = match reduce_op {
            ReduceOp::Sum => partials.iter().sum(),
            ReduceOp::Max => partials.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ReduceOp::Min => partials.iter().cloned().fold(f64::INFINITY, f64::min),
            ReduceOp::Product => partials.iter().sum::<f64>(), // log-domain sum
        };

        Ok(result)
    }

    /// Read single f64 result from buffer
    fn read_result(&self, buffer: &wgpu::Buffer) -> Result<f64> {
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FMR Result Staging"),
            size: 8,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("FMR Result Encoder"),
                });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, 8);
        self.device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let result = f64::from_le_bytes(data[..8].try_into().expect("scalar buffer is 8 bytes"));
        drop(data);
        staging.unmap();

        Ok(result)
    }

    /// CPU fallback for small arrays (faster due to no dispatch overhead)
    #[cfg(test)]
    fn execute_cpu(
        &self,
        data: &[f64],
        total: f64,
        map_op: MapOp,
        reduce_op: ReduceOp,
    ) -> Result<f64> {
        let mapped: Vec<f64> = data
            .iter()
            .map(|&x| match map_op {
                MapOp::Identity => x,
                MapOp::Shannon => {
                    if x <= 0.0 {
                        0.0
                    } else {
                        let p = x / total;
                        if p <= 1e-300 {
                            0.0
                        } else {
                            -p * p.ln()
                        }
                    }
                }
                MapOp::Simpson => {
                    let p = x / total;
                    p * p
                }
                MapOp::Square => x * x,
                MapOp::Abs => x.abs(),
                MapOp::Log => x.ln(),
                MapOp::Negate => -x,
            })
            .collect();

        let result = match reduce_op {
            ReduceOp::Sum => mapped.iter().sum(),
            ReduceOp::Max => mapped.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ReduceOp::Min => mapped.iter().cloned().fold(f64::INFINITY, f64::min),
            ReduceOp::Product => mapped.iter().map(|x| x.ln()).sum::<f64>().exp(),
        };

        Ok(result)
    }

    // ========================================================================
    // CONVENIENCE METHODS — Common patterns for all springs
    // ========================================================================

    /// Shannon entropy: H = -Σ p * log(p) where p = count / total
    ///
    /// wetSpring primary use case for metagenomics diversity.
    pub fn shannon_entropy(&self, counts: &[f64]) -> Result<f64> {
        let total: f64 = counts.iter().sum();
        if total <= 0.0 {
            return Ok(0.0);
        }
        self.execute(counts, total, MapOp::Shannon, ReduceOp::Sum)
    }

    /// Simpson index: D = Σ p² where p = count / total
    ///
    /// wetSpring use case for diversity measurement.
    pub fn simpson_index(&self, counts: &[f64]) -> Result<f64> {
        let total: f64 = counts.iter().sum();
        if total <= 0.0 {
            return Ok(0.0);
        }
        self.execute(counts, total, MapOp::Simpson, ReduceOp::Sum)
    }

    /// Sum of squares: Σ x²
    ///
    /// Common for L2 norms, variance calculations.
    pub fn sum_of_squares(&self, data: &[f64]) -> Result<f64> {
        self.execute(data, 1.0, MapOp::Square, ReduceOp::Sum)
    }

    /// L1 norm: Σ |x|
    pub fn l1_norm(&self, data: &[f64]) -> Result<f64> {
        self.execute(data, 1.0, MapOp::Abs, ReduceOp::Sum)
    }

    /// Maximum value
    pub fn max(&self, data: &[f64]) -> Result<f64> {
        self.execute(data, 1.0, MapOp::Identity, ReduceOp::Max)
    }

    /// Minimum value
    pub fn min(&self, data: &[f64]) -> Result<f64> {
        self.execute(data, 1.0, MapOp::Identity, ReduceOp::Min)
    }

    /// Sum
    pub fn sum(&self, data: &[f64]) -> Result<f64> {
        self.execute(data, 1.0, MapOp::Identity, ReduceOp::Sum)
    }

    /// Convenience method for dot product of two vectors.
    ///
    /// Computes Σ a[i] * b[i] using the sum-of-products pattern:
    /// element-wise products are formed on the host, then reduced on GPU.
    pub fn dot(&self, a: &[f64], b: &[f64]) -> Result<f64> {
        if a.len() != b.len() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!("dot: length mismatch ({} vs {})", a.len(), b.len()),
            });
        }
        let products: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
        self.sum(&products)
    }
}

#[cfg(test)]
#[path = "fused_map_reduce_f64_tests.rs"]
mod tests;
