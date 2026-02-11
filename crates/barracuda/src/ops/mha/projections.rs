//! GPU projection operations for Multi-Head Attention
//!
//! This module contains the WGSL-based projection operations that transform
//! input tensors through learned weight matrices with head splitting/concatenation.

use super::{MhaParams, MultiHeadAttention};
use crate::error::Result;
use crate::tensor::Tensor;

impl MultiHeadAttention {
    /// Projection shader: [B,S,D] + [D,D] → [B,H,S,D/H]
    pub(super) fn shader_projection() -> &'static str {
        include_str!("../../shaders/attention/mha_projection.wgsl")
    }

    /// Output shader: [B,H,S,D/H] + [D,D] → [B,S,D]
    pub(super) fn shader_output() -> &'static str {
        include_str!("../../shaders/tensor/mha_output.wgsl")
    }

    /// Project input through weight with head splitting
    /// [B, S, D] + [D, D] → [B, H, S, D/H]
    pub(super) fn project_with_head_split(
        &self,
        input: &Tensor,
        weight: &Tensor,
        params: &MhaParams,
    ) -> Result<Tensor> {
        let device = input.device();

        // Output size: [B, H, S, D/H]
        let output_size =
            (params.batch_size * params.num_heads * params.seq_len * params.head_dim) as usize;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params buffer
        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MHA Projection Params"),
            size: std::mem::size_of::<MhaParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(params));

        // Compile shader
        let shader = device.compile_shader(Self::shader_projection(), Some("MHA Projection"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MHA Projection BGL"),
                entries: &[
                    // Input
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
                    // Weight
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
                    // Output
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
                    // Params
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
            label: Some("MHA Projection BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MHA Projection Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MHA Projection Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and dispatch
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MHA Projection Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MHA Projection Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: one thread per (batch, head, seq) - each processes head_dim
            let workgroups_x = params.batch_size.div_ceil(16);
            let workgroups_y = params.num_heads.div_ceil(16);
            let workgroups_z = params.seq_len.div_ceil(16);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Create output tensor: [B, H, S, D/H]
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![
                params.batch_size as usize,
                params.num_heads as usize,
                params.seq_len as usize,
                params.head_dim as usize,
            ],
            device.clone(),
        ))
    }

    /// Concatenate heads and project through output weight
    /// [B, H, S, D/H] + [D, D] → [B, S, D]
    pub(super) fn concat_and_project(
        &self,
        attention_out: &Tensor,
        w_o: &Tensor,
        params: &MhaParams,
    ) -> Result<Tensor> {
        let device = attention_out.device();

        // Output size: [B, S, D]
        let output_size = (params.batch_size * params.seq_len * params.d_model) as usize;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params buffer
        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MHA Output Params"),
            size: std::mem::size_of::<MhaParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(params));

        // Compile shader
        let shader = device.compile_shader(Self::shader_output(), Some("MHA Output"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MHA Output BGL"),
                entries: &[
                    // Attention output (heads)
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
                    // Output weight
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
                    // Output
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
                    // Params
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
            label: Some("MHA Output BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: attention_out.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: w_o.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("MHA Output Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MHA Output Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and dispatch
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MHA Output Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MHA Output Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: one thread per (batch, seq, output_dim)
            let workgroups_x = params.batch_size.div_ceil(16);
            let workgroups_y = params.seq_len.div_ceil(16);
            let workgroups_z = params.d_model.div_ceil(16);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Create output tensor: [B, S, D]
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![
                params.batch_size as usize,
                params.seq_len as usize,
                params.d_model as usize,
            ],
            device.clone(),
        ))
    }
}
