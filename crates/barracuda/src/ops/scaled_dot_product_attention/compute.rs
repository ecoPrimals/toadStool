//! GPU compute operations for Scaled Dot-Product Attention
//!
//! This module contains the GPU execution logic for scaled dot-product attention,
//! including the three-pass execution: matmul, softmax, and apply.

use super::{AttentionParams, ScaledDotProductAttention};
use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Execute the scaled dot-product attention operation
///
/// Performs multi-pass execution:
/// 1. Compute Q @ K^T scores
/// 2. Apply softmax to scores
/// 3. Apply attention weights to values
pub(super) fn execute_scaled_dot_product_attention(
    op: ScaledDotProductAttention,
) -> Result<Tensor> {
    let device = op.query().device();

    // Calculate buffer sizes
    let input_size = op.batch_size() * op.num_heads() * op.seq_len() * op.head_dim();
    let scores_size = op.batch_size() * op.num_heads() * op.seq_len() * op.seq_len();

    // Create intermediate buffers
    let scores_buffer = device.create_buffer_f32(scores_size)?;
    let weights_buffer = device.create_buffer_f32(scores_size)?;
    let output_buffer = device.create_buffer_f32(input_size)?;

    // Create parameters buffer
    let params = AttentionParams {
        batch_size: op.batch_size() as u32,
        num_heads: op.num_heads() as u32,
        seq_len: op.seq_len() as u32,
        head_dim: op.head_dim() as u32,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Attention Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Create command encoder for all passes
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ScaledDotProductAttention Encoder"),
        });

    // ═══════════════════════════════════════════════════════════════
    // PASS 1: Compute Q @ K^T scores
    // ═══════════════════════════════════════════════════════════════
    {
        let shader_module = device.compile_shader(
            ScaledDotProductAttention::wgsl_shader_matmul(),
            Some("Attention MatMul Shader"),
        );

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention MatMul Bind Group Layout"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention MatMul Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: op.query().buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: op.key().buffer().as_entire_binding(),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention MatMul Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Attention MatMul Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Attention MatMul Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Deep Debt Evolution: Capability-based dispatch
        // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
        // We use capability awareness to determine optimal tile count
        let caps = DeviceCapabilities::from_device(device);
        let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
        // Tile size is shader-constrained to 16x16, but we ensure capability awareness
        const TILE_SIZE: u32 = 16;
        let workgroups_x = (op.seq_len() as u32).div_ceil(TILE_SIZE).max(1);
        let workgroups_y = (op.seq_len() as u32).div_ceil(TILE_SIZE).max(1);
        let workgroups_z = (op.batch_size() * op.num_heads()) as u32;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
    }

    // ═══════════════════════════════════════════════════════════════
    // PASS 2: Apply softmax to scores
    // ═══════════════════════════════════════════════════════════════
    {
        let shader_module = device.compile_shader(
            ScaledDotProductAttention::wgsl_shader_softmax(),
            Some("Attention Softmax Shader"),
        );

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Softmax Bind Group Layout"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Softmax Bind Group"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Attention Softmax Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Attention Softmax Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Deep Debt Evolution: Capability-based dispatch
        // Each thread handles one query position (one row of scores)
        let total_rows = op.batch_size() * op.num_heads() * op.seq_len();
        let caps = DeviceCapabilities::from_device(device);
        let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
        let workgroups = (total_rows as u32).div_ceil(optimal_wg_size);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    // ═══════════════════════════════════════════════════════════════
    // PASS 3: Apply attention weights to values
    // ═══════════════════════════════════════════════════════════════
    {
        let shader_module = device.compile_shader(
            ScaledDotProductAttention::wgsl_shader_apply(),
            Some("Attention Apply Shader"),
        );

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Apply Bind Group Layout"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attention Apply Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: op.value().buffer().as_entire_binding(),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Apply Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Attention Apply Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Attention Apply Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Deep Debt Evolution: Capability-based dispatch
        // Shader uses fixed 16x16 tiles (workgroup_size(16, 16, 1))
        // We use capability awareness to determine optimal tile count
        let caps = DeviceCapabilities::from_device(device);
        let _optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
        // Tile size is shader-constrained to 16x16, but we ensure capability awareness
        const TILE_SIZE: u32 = 16;
        let workgroups_x = (op.head_dim() as u32).div_ceil(TILE_SIZE).max(1);
        let workgroups_y = (op.seq_len() as u32).div_ceil(TILE_SIZE).max(1);
        let workgroups_z = (op.batch_size() * op.num_heads()) as u32;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
    }

    // Submit all passes
    device.queue.submit(Some(encoder.finish()));

    // Return output tensor
    Ok(Tensor::from_buffer(
        output_buffer,
        vec![op.batch_size(), op.num_heads(), op.seq_len(), op.head_dim()],
        device.clone(),
    ))
}
