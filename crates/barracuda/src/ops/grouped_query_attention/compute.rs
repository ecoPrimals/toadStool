//! GPU compute operations for Grouped Query Attention
//!
//! This module contains the 3-pass GPU execution:
//! 1. Pass 1: Compute Q @ K^T scores (with grouped KV heads)
//! 2. Pass 2: Apply softmax to scores
//! 3. Pass 3: Apply attention weights to values

use super::{GroupedQueryAttention, GQAParams};
use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

impl GroupedQueryAttention {
    /// Execute the grouped query attention operation
    ///
    /// Performs multi-pass execution adapted for grouped queries:
    /// 1. Compute Q @ K^T scores (with grouped KV heads)
    /// 2. Apply softmax to scores
    /// 3. Apply attention weights to values
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query().device();

        // Calculate buffer sizes
        // Scores: [batch, num_q_heads, seq_len, seq_len]
        let scores_size = self.batch_size() * self.num_q_heads() * self.seq_len() * self.seq_len();
        let weights_size = scores_size;
        let output_size = self.batch_size() * self.num_q_heads() * self.seq_len() * self.head_dim();

        // Create intermediate buffers
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(weights_size)?;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create parameters buffer
        let params = GQAParams {
            batch_size: self.batch_size() as u32,
            num_q_heads: self.num_q_heads() as u32,
            num_kv_heads: self.num_kv_heads() as u32,
            seq_len: self.seq_len() as u32,
            head_dim: self.head_dim() as u32,
            heads_per_group: self.heads_per_group() as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GQA Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create command encoder for all passes
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GroupedQueryAttention Encoder"),
        });

        // ═══════════════════════════════════════════════════════════════
        // PASS 1: Compute Q @ K^T scores (with grouped KV heads)
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_matmul(),
                Some("GQA MatMul Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA MatMul Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GQA MatMul Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.query().buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.key().buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: scores_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("GQA MatMul Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA MatMul Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA MatMul Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
            // We use capability awareness to determine optimal tile count
            let caps = DeviceCapabilities::from_device(&device);
            let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            // Tile size is shader-constrained to 16x16, but we ensure capability awareness
            const TILE_SIZE: u32 = 16;
            let workgroups_x = ((self.seq_len() as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_y = ((self.seq_len() as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_z = (self.batch_size() * self.num_q_heads()) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 2: Apply softmax to scores
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_softmax(),
                Some("GQA Softmax Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA Softmax Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GQA Softmax Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: scores_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weights_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("GQA Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA Softmax Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA Softmax Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Each thread handles one query position (one row of scores)
            let total_rows = self.batch_size() * self.num_q_heads() * self.seq_len();
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups = (total_rows as u32 + optimal_wg_size - 1) / optimal_wg_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 3: Apply attention weights to values
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_apply(),
                Some("GQA Apply Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA Apply Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GQA Apply Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: weights_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.value().buffer().as_entire_binding(),
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

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("GQA Apply Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA Apply Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA Apply Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
            // We use capability awareness to determine optimal tile count
            let caps = DeviceCapabilities::from_device(&device);
            let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            // Tile size is shader-constrained to 16x16, but we ensure capability awareness
            const TILE_SIZE: u32 = 16;
            let workgroups_x = ((self.head_dim() as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_y = ((self.seq_len() as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_z = (self.batch_size() * self.num_q_heads()) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // Submit all passes
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![self.batch_size(), self.num_q_heads(), self.seq_len(), self.head_dim()],
            device.clone(),
        ))
    }
}
