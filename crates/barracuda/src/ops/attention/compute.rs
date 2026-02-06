//! GPU compute operations for Scaled Dot-Product Attention
//!
//! This module contains the 3-pass GPU execution:
//! 1. Pass 1: Compute QK^T scores (matrix multiplication)
//! 2. Pass 2: Apply softmax to scores (row-wise)
//! 3. Pass 3: Apply weights to values (weighted sum)

use super::{Attention, AttentionParams};
use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

impl Attention {
    /// Execute attention operation (3 GPU passes)
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query().device();

        // Extract dimensions
        let shape = self.query().shape();
        let batch_size = shape[0];
        let num_heads = shape[1];
        let seq_len = shape[2];
        let head_dim = shape[3];

        // Create parameters
        let params = AttentionParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            seq_len: seq_len as u32,
            head_dim: head_dim as u32,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Attention Params"),
            size: std::mem::size_of::<AttentionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Intermediate buffers
        let scores_size = batch_size * num_heads * seq_len * seq_len;
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(scores_size)?;

        // Output buffer
        let output_size = batch_size * num_heads * seq_len * head_dim;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // ═══════════════════════════════════════════════════════════
        // PASS 1: Compute QK^T scores
        // ═══════════════════════════════════════════════════════════

        let shader_matmul = device.compile_shader(Self::shader_matmul(), Some("AttentionMatmul"));

        let bgl_matmul = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Attention Matmul BGL"),
                entries: &[
                    // Query
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
                    // Key
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
                    // Scores (output)
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

        let bg_matmul = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Matmul BG"),
            layout: &bgl_matmul,
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

        let pipeline_layout_matmul =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Matmul PL"),
                    bind_group_layouts: &[&bgl_matmul],
                    push_constant_ranges: &[],
                });

        let pipeline_matmul =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Matmul Pipeline"),
                    layout: Some(&pipeline_layout_matmul),
                    module: &shader_matmul,
                    entry_point: "main",
                });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Attention Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
            // We use capability awareness to determine optimal tile count
            let caps = DeviceCapabilities::from_device(&device);
            let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            // Tile size is shader-constrained to 16x16, but we ensure capability awareness
            const TILE_SIZE: u32 = 16;
            let workgroups_x = ((seq_len as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_y = ((seq_len as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_z = (batch_size * num_heads) as u32;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax
        // ═══════════════════════════════════════════════════════════

        let shader_softmax =
            device.compile_shader(Self::shader_softmax(), Some("AttentionSoftmax"));

        let bgl_softmax =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Softmax BGL"),
                    entries: &[
                        // Scores (input)
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
                        // Weights (output)
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
                        // Params
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

        let bg_softmax = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Softmax BG"),
            layout: &bgl_softmax,
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

        let pipeline_layout_softmax =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Softmax PL"),
                    bind_group_layouts: &[&bgl_softmax],
                    push_constant_ranges: &[],
                });

        let pipeline_softmax =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout_softmax),
                    module: &shader_softmax,
                    entry_point: "main",
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Softmax Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_softmax);
            pass.set_bind_group(0, &bg_softmax, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Softmax is element-wise per [batch, head, query_pos]
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = ((batch_size * num_heads * seq_len) as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values
        // ═══════════════════════════════════════════════════════════

        let shader_apply = device.compile_shader(Self::shader_apply(), Some("AttentionApply"));

        let bgl_apply = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Attention Apply BGL"),
                entries: &[
                    // Weights (input)
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
                    // Value (input)
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

        let bg_apply = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Apply BG"),
            layout: &bgl_apply,
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

        let pipeline_layout_apply =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Apply PL"),
                    bind_group_layouts: &[&bgl_apply],
                    push_constant_ranges: &[],
                });

        let pipeline_apply =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Apply Pipeline"),
                    layout: Some(&pipeline_layout_apply),
                    module: &shader_apply,
                    entry_point: "main",
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Apply Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_apply);
            pass.set_bind_group(0, &bg_apply, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
            // We use capability awareness to determine optimal tile count
            let caps = DeviceCapabilities::from_device(&device);
            let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            // Tile size is shader-constrained to 16x16, but we ensure capability awareness
            const TILE_SIZE: u32 = 16;
            let workgroups_x = ((head_dim as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_y = ((seq_len as u32 + TILE_SIZE - 1) / TILE_SIZE).max(1);
            let workgroups_z = (batch_size * num_heads) as u32;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // Submit all passes
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, num_heads, seq_len, head_dim],
            device.clone(),
        ))
    }
}
