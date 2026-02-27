//! GPU compute operations for Cross Attention
//!
//! This module contains the 3-pass GPU execution:
//! 1. Pass 1: Compute QK^T scores (matmul)
//! 2. Pass 2: Apply softmax normalization
//! 3. Pass 3: Apply weights to values

use super::{CrossAttention, CrossAttentionParams};
use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

impl CrossAttention {
    /// Execute cross attention (3 GPU passes)
    ///
    /// **Deep Debt**: Custom WGSL handles asymmetric seq_lens
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query().device();

        // Extract dimensions
        let q_shape = self.query().shape();
        let k_shape = self.key().shape();

        let batch_size = q_shape[0];
        let num_heads = q_shape[1];
        let decoder_seq = q_shape[2];
        let encoder_seq = k_shape[2]; // Different from decoder!
        let head_dim = q_shape[3];

        // Create parameters
        let params = CrossAttentionParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            decoder_seq: decoder_seq as u32,
            encoder_seq: encoder_seq as u32,
            head_dim: head_dim as u32,
            _padding: [0, 0, 0],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cross Attention Params"),
            size: std::mem::size_of::<CrossAttentionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Intermediate buffers
        let scores_size = batch_size * num_heads * decoder_seq * encoder_seq;
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(scores_size)?;

        // Output buffer [B, H, Dec, D]
        let output_size = batch_size * num_heads * decoder_seq * head_dim;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // ═══════════════════════════════════════════════════════════
        // PASS 1: Compute QK^T scores
        // ═══════════════════════════════════════════════════════════

        let shader_matmul =
            device.compile_shader(Self::shader_matmul(), Some("CrossAttentionMatmul"));

        let bgl_matmul = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cross Attention Matmul BGL"),
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

        let bg_matmul = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cross Attention Matmul BG"),
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
                    label: Some("Cross Attention Matmul Pipeline Layout"),
                    bind_group_layouts: &[&bgl_matmul],
                    push_constant_ranges: &[],
                });

        let pipeline_matmul =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Cross Attention Matmul Pipeline"),
                    layout: Some(&pipeline_layout_matmul),
                    module: &shader_matmul,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax
        // ═══════════════════════════════════════════════════════════

        let shader_softmax =
            device.compile_shader(Self::shader_softmax(), Some("CrossAttentionSoftmax"));

        let bgl_softmax =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Cross Attention Softmax BGL"),
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

        let bg_softmax = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cross Attention Softmax BG"),
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
                    label: Some("Cross Attention Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bgl_softmax],
                    push_constant_ranges: &[],
                });

        let pipeline_softmax =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Cross Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout_softmax),
                    module: &shader_softmax,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values
        // ═══════════════════════════════════════════════════════════

        let shader_apply = device.compile_shader(Self::shader_apply(), Some("CrossAttentionApply"));

        let bgl_apply = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cross Attention Apply BGL"),
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

        let bg_apply = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cross Attention Apply BG"),
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
                    label: Some("Cross Attention Apply Pipeline Layout"),
                    bind_group_layouts: &[&bgl_apply],
                    push_constant_ranges: &[],
                });

        let pipeline_apply =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Cross Attention Apply Pipeline"),
                    layout: Some(&pipeline_layout_apply),
                    module: &shader_apply,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // ═══════════════════════════════════════════════════════════
        // EXECUTE ALL 3 PASSES
        // ═══════════════════════════════════════════════════════════

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cross Attention Encoder"),
            });

        // Pass 1: Matmul
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cross Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            // MatMul is element-wise per score element
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups = ((batch_size * num_heads * decoder_seq * encoder_seq) as u32)
                .div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }

        // Pass 2: Softmax
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cross Attention Softmax Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_softmax);
            pass.set_bind_group(0, &bg_softmax, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups =
                ((batch_size * num_heads * decoder_seq) as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 3: Apply
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cross Attention Apply Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_apply);
            pass.set_bind_group(0, &bg_apply, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            // Apply is element-wise per output element
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = ((batch_size * num_heads * decoder_seq * head_dim) as u32)
                .div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }

        device.submit_and_poll(Some(encoder.finish()));

        // Return output tensor [B, H, Dec, D]
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, num_heads, decoder_seq, head_dim],
            device.clone(),
        ))
    }
}
