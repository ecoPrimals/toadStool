// SPDX-License-Identifier: AGPL-3.0-only

//! Batched multinomial sampling for rarefaction — GPU kernel.
//!
//! Each GPU thread runs one replicate: draws `depth` reads from a community
//! described by cumulative abundance probabilities, counting how many reads
//! land in each taxon via binary search.
//!
//! Uses xoshiro128** PRNG matching `barracuda::ops::prng_xoshiro_wgsl`.
//!
//! Provenance: groundSpring metalForge → toadStool absorption

use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::device::WgpuDevice;
use crate::error::Result;

/// WGSL shader source for batched multinomial sampling (f64 probabilities).
pub const WGSL_BATCHED_MULTINOMIAL_F64: &str =
    include_str!("../../shaders/bio/batched_multinomial_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParams {
    n_taxa: u32,
    depth: u32,
    n_reps: u32,
    _pad: u32,
}

/// GPU-backed batched multinomial sampling for rarefaction.
pub struct BatchedMultinomialGpu {
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    device: Arc<WgpuDevice>,
}

impl BatchedMultinomialGpu {
    /// Compile the multinomial sampling shader.
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let d = device.device();
        let module =
            device.compile_shader_f64(WGSL_BATCHED_MULTINOMIAL_F64, Some("BatchedMultinomial"));

        let bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BatchedMultinomial BGL"),
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

        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BatchedMultinomial Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("BatchedMultinomial Pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(Self {
            pipeline,
            bgl,
            device,
        })
    }

    /// Draw `depth` multinomial samples for each of `n_reps` replicates.
    ///
    /// `cumulative_probs` holds the cumulative probability for each taxon (length `n_taxa`).
    /// `seeds` holds `n_reps * 4` u32 values (xoshiro128** state per replicate).
    ///
    /// Returns `counts[n_reps][n_taxa]` flattened row-major.
    pub fn sample(
        &self,
        cumulative_probs: &[f64],
        seeds: &mut Vec<u32>,
        depth: u32,
        n_reps: u32,
    ) -> Result<Vec<u32>> {
        let n_taxa = cumulative_probs.len();
        assert_eq!(seeds.len(), n_reps as usize * 4);

        let d = self.device.device();
        let q = self.device.queue();

        let params = GpuParams {
            n_taxa: n_taxa as u32,
            depth,
            n_reps,
            _pad: 0,
        };

        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BatchedMultinomial params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let cumul_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BatchedMultinomial cumulative"),
            contents: bytemuck::cast_slice(cumulative_probs),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let seeds_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BatchedMultinomial seeds"),
            contents: bytemuck::cast_slice(seeds.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let counts_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedMultinomial counts"),
            size: (n_reps as usize * n_taxa * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BatchedMultinomial BG"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cumul_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: seeds_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: counts_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = d.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("BatchedMultinomial Encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BatchedMultinomial Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(n_reps.div_ceil(64), 1, 1);
        }
        q.submit(std::iter::once(encoder.finish()));
        d.poll(wgpu::Maintain::Wait);

        let counts = self
            .device
            .read_buffer_u32(&counts_buf, n_reps as usize * n_taxa)?;

        Ok(counts)
    }
}

/// CPU reference: multinomial sampling for rarefaction.
///
/// `cumulative_probs` must be monotonically non-decreasing, ending near 1.0.
/// Returns `counts[n_taxa]` for a single replicate.
#[must_use]
pub fn multinomial_sample_cpu(
    cumulative_probs: &[f64],
    depth: u32,
    rng: &mut impl FnMut() -> f64,
) -> Vec<u32> {
    let n_taxa = cumulative_probs.len();
    let mut counts = vec![0u32; n_taxa];
    for _ in 0..depth {
        let u = rng();
        let taxon = cumulative_probs
            .partition_point(|&c| c < u)
            .min(n_taxa - 1);
        counts[taxon] += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[test]
    fn cpu_total_equals_depth() {
        let cumul = vec![0.25, 0.50, 0.75, 1.0];
        let mut counter = 0u64;
        let mut rng = || {
            counter += 1;
            (counter as f64 * 0.1234567) % 1.0
        };
        let counts = multinomial_sample_cpu(&cumul, 1000, &mut rng);
        let total: u32 = counts.iter().sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn cpu_single_taxon() {
        let cumul = vec![1.0];
        let mut rng = || 0.5;
        let counts = multinomial_sample_cpu(&cumul, 100, &mut rng);
        assert_eq!(counts, vec![100]);
    }

    #[test]
    fn cpu_deterministic_ordering() {
        let cumul = vec![0.0, 0.0, 1.0];
        let mut rng = || 0.5;
        let counts = multinomial_sample_cpu(&cumul, 50, &mut rng);
        assert_eq!(counts[0], 0);
        assert_eq!(counts[1], 0);
        assert_eq!(counts[2], 50);
    }

    #[tokio::test]
    async fn gpu_total_equals_depth() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };

        let gpu = match BatchedMultinomialGpu::new(device) {
            Ok(g) => g,
            Err(_) => return,
        };

        let cumul = vec![0.25, 0.50, 0.75, 1.0];
        let n_reps = 8u32;
        let depth = 500u32;
        let mut seeds: Vec<u32> = (0..n_reps * 4).map(|i| 42 + i * 7).collect();

        let counts = gpu.sample(&cumul, &mut seeds, depth, n_reps).unwrap();
        assert_eq!(counts.len(), n_reps as usize * cumul.len());

        for rep in 0..n_reps as usize {
            let row = &counts[rep * cumul.len()..(rep + 1) * cumul.len()];
            let total: u32 = row.iter().sum();
            assert_eq!(
                total, depth,
                "replicate {rep}: sum={total}, expected {depth}"
            );
        }
    }
}
