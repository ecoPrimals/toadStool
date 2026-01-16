//! Batch Normalization
//!
//! Normalizes activations across the batch dimension for stable training.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    pub async fn execute_batchnorm(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
        config: BatchNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;

        anyhow::ensure!(
            input.len() == total_size,
            "BatchNorm: input size must equal batch_size * channels * spatial_size"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "BatchNorm: gamma size must equal channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "BatchNorm: beta size must equal channels"
        );
        anyhow::ensure!(
            config.running_mean.len() == channels,
            "BatchNorm: running_mean size must equal channels"
        );
        anyhow::ensure!(
            config.running_var.len() == channels,
            "BatchNorm: running_var size must equal channels"
        );

        let shader_source = include_str!("../../shaders/batchnorm.wgsl");

        // Create input buffers
        let input_buffer = self.create_input_buffer(input, "BatchNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "BatchNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "BatchNorm Beta");
        let mean_buffer = self.create_input_buffer(&config.running_mean, "BatchNorm Mean");
        let var_buffer = self.create_input_buffer(&config.running_var, "BatchNorm Var");
        let output_buffer = self.create_output_buffer(total_size, "BatchNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "BatchNorm Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BatchNormParams {
            batch_size: u32,
            channels: u32,
            spatial_size: u32,
            epsilon: f32,
            training: u32,
            _padding: [u32; 3],
        }

        let params = BatchNormParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            epsilon: config.epsilon,
            training: 0, // Inference mode (Deep Debt: configurable!)
            _padding: [0; 3],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BatchNorm Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Complex bind group with 7 bindings
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BatchNorm Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
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
            label: Some("BatchNorm Bind Group"),
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
                    resource: mean_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: var_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "BatchNorm", &bind_group_layout);
        let workgroups = self.calculate_workgroups(total_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "BatchNorm");

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
