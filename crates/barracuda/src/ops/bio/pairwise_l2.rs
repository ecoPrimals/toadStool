// SPDX-License-Identifier: AGPL-3.0-only

//! Pairwise L2 Distance — GPU kernel.
//!
//! Computes the upper-triangle pairwise Euclidean (L2) distance matrix for N
//! feature vectors of dimension D. Each thread handles one pair. Output is
//! N*(N-1)/2 L2 distances.
//!
//! Provenance: neuralSpring metalForge → toadStool absorption

use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::device::WgpuDevice;

pub const WGSL_PAIRWISE_L2: &str = include_str!("../../shaders/math/pairwise_l2.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PairwiseL2Params {
    n: u32,
    dim: u32,
}

pub struct PairwiseL2Gpu {
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    device: Arc<WgpuDevice>,
}

impl PairwiseL2Gpu {
    pub fn new(device: Arc<WgpuDevice>) -> Self {
        let d = device.device();

        let bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PairwiseL2 BGL"),
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

        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PairwiseL2 Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PairwiseL2 Shader"),
            source: wgpu::ShaderSource::Wgsl(WGSL_PAIRWISE_L2.into()),
        });

        let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PairwiseL2 Pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bgl,
            device,
        }
    }

    /// Compute pairwise L2 distances for `n` vectors of dimension `dim`.
    ///
    /// `input_buf`: `[n × dim]` f32 (row-major feature vectors)
    /// `output_buf`: `[n*(n-1)/2]` f32 (L2 distances)
    pub fn dispatch(&self, input_buf: &wgpu::Buffer, output_buf: &wgpu::Buffer, n: u32, dim: u32) {
        let d = self.device.device();
        let q = self.device.queue();

        let params = PairwiseL2Params { n, dim };
        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PairwiseL2 Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let n_pairs = n * (n - 1) / 2;

        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PairwiseL2 BG"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = d.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PairwiseL2 Encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PairwiseL2 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(n_pairs.div_ceil(256), 1, 1);
        }
        q.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::{PairwiseL2Gpu, WGSL_PAIRWISE_L2};

    #[test]
    fn sanity_constants_exported() {
        assert!(!WGSL_PAIRWISE_L2.is_empty());
        assert!(WGSL_PAIRWISE_L2.contains("fn main"));
        assert!(WGSL_PAIRWISE_L2.contains("PairwiseParams"));
        assert!(std::any::type_name::<PairwiseL2Gpu>().contains("PairwiseL2Gpu"));
    }
}
