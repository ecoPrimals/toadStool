//! Instance Normalization
//!
//! Normalizes each instance independently, common in style transfer.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    pub async fn execute_instance_norm(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        spatial_size: usize, // height * width
        config: InstanceNormConfig,
    ) -> Result<Vec<f32>> {
        let total_size = batch * channels * spatial_size;
        anyhow::ensure!(
            input.len() == total_size,
            "InstanceNorm: input size must match batch * channels * spatial_size"
        );
        anyhow::ensure!(
            config.gamma.len() == channels,
            "InstanceNorm: gamma size must match channels"
        );
        anyhow::ensure!(
            config.beta.len() == channels,
            "InstanceNorm: beta size must match channels"
        );

        let shader_source = include_str!("../../shaders/instancenorm.wgsl");

        let input_buffer = self.create_input_buffer(input, "InstanceNorm Input");
        let gamma_buffer = self.create_input_buffer(&config.gamma, "InstanceNorm Gamma");
        let beta_buffer = self.create_input_buffer(&config.beta, "InstanceNorm Beta");
        let output_buffer = self.create_output_buffer(total_size, "InstanceNorm Output");
        let staging_buffer = self.create_staging_buffer(total_size, "InstanceNorm Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct InstanceNormParams {
            batch: u32,
            channels: u32,
            spatial_size: u32,
            epsilon: f32,
        }

        let params = InstanceNormParams {
            batch: batch as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            epsilon: config.epsilon,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("InstanceNorm Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("InstanceNorm Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InstanceNorm Bind Group"),
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
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "InstanceNorm", &bind_group_layout);

        let num_instances = batch * channels;
        let workgroups = self.calculate_workgroups(num_instances, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "InstanceNorm");

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

    /// Execute RMS Normalization
    ///
    /// Simpler alternative to LayerNorm used in modern transformers.
    /// RMSNorm(x) = x / sqrt(mean(x²) + epsilon) * gamma
    ///
    /// No mean subtraction, only RMS scaling - faster and simpler than LayerNorm.
    /// Used in: LLaMA, GPT-NeoX, T5, modern large language models.
    ///
    /// Deep Debt: Runtime dimensions, learnable scale parameters.
}
