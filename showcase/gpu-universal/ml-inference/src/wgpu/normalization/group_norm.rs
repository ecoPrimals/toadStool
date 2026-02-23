//! Group normalization
//!
//! Normalize within groups of channels.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    pub async fn execute_groupnorm(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
        config: GroupNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;

        anyhow::ensure!(
            input.len() == total_size,
            "GroupNorm: input size must equal batch_size * channels * spatial_size"
        );
        anyhow::ensure!(
            channels.is_multiple_of(config.num_groups),
            "GroupNorm: channels must be divisible by num_groups"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "GroupNorm: gamma size must equal channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "GroupNorm: beta size must equal channels"
        );

        let channels_per_group = channels / config.num_groups;
        let total_groups = batch_size * config.num_groups;

        let shader_source = include_str!("../../shaders/groupnorm.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "GroupNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "GroupNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "GroupNorm Beta");
        let output_buffer = self.create_output_buffer(total_size, "GroupNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "GroupNorm Staging");

        // Statistics buffer: 2 values (mean, variance) per group
        let stats_size = total_groups * 2;
        let stats_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GroupNorm Stats"),
            size: (stats_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GroupNormParams {
            batch_size: u32,
            channels: u32,
            spatial_size: u32,
            num_groups: u32,
            channels_per_group: u32,
            epsilon: f32,
            _padding: [u32; 2],
        }

        let params = GroupNormParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            num_groups: config.num_groups as u32,
            channels_per_group: channels_per_group as u32,
            epsilon: config.epsilon,
            _padding: [0; 2],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GroupNorm Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Complex bind group with 6 bindings
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GroupNorm Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GroupNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gamma_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: beta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipelines for multi-pass algorithm
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("GroupNorm Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GroupNorm Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Two passes: compute_stats, then normalize
        let compute_stats_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("GroupNorm Compute Stats"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "compute_stats",
                    compilation_options: Default::default(),
                    cache: None,
                });

        let normalize_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("GroupNorm Normalize"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "normalize",
                    compilation_options: Default::default(),
                    cache: None,
                });

        // Execute two-pass algorithm
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GroupNorm Encoder"),
            });

        // Pass 1: Compute group statistics
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GroupNorm Compute Stats"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&compute_stats_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // One workgroup per group
            pass.dispatch_workgroups(1, 1, total_groups as u32);
        }

        // Pass 2: Normalize using computed statistics
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GroupNorm Normalize"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&normalize_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = self.calculate_workgroups(total_size, 256);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (total_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, total_size).await
    }
}
