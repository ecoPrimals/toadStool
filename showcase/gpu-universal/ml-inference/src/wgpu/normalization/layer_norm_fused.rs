// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fused Layer Normalization
//!

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    pub async fn execute_layernorm_fused(
        &self,
        input: &[f32],
        config: NormConfig,
    ) -> Result<Vec<f32>> {
        let size = input.len();
        anyhow::ensure!(size > 0, "LayerNorm Fused: input cannot be empty");

        // Calculate workgroups with cap for large inputs (grid-stride handles the rest)
        let workgroups = self.calculate_workgroups(size, 256).max(1).min(65535);
        let shader_source = include_str!("../../shaders/layernorm_fused.wgsl");

        // Create buffers (NO stats buffer needed - everything in shared memory!)
        let input_buffer = self.create_input_buffer(input, "LayerNorm Fused Input");

        let gamma = config.gamma.unwrap_or_else(|| vec![1.0; size]);
        anyhow::ensure!(
            gamma.len() == size,
            "LayerNorm Fused: gamma size must match input size"
        );
        let gamma_buffer = self.create_input_buffer(&gamma, "LayerNorm Fused Gamma");

        let beta = config.beta.unwrap_or_else(|| vec![0.0; size]);
        anyhow::ensure!(
            beta.len() == size,
            "LayerNorm Fused: beta size must match input size"
        );
        let beta_buffer = self.create_input_buffer(&beta, "LayerNorm Fused Beta");

        let output_buffer = self.create_output_buffer(size, "LayerNorm Fused Output");
        let staging_buffer = self.create_staging_buffer(size, "LayerNorm Fused Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct LayerNormParams {
            size: u32,
            epsilon: f32,
        }

        let params = LayerNormParams {
            size: size as u32,
            epsilon: config.epsilon,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LayerNorm Fused Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Simplified bind group layout (5 bindings instead of 6 - no stats buffer!)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LayerNorm Fused Layout"),
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
            label: Some("LayerNorm Fused Bind Group"),
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

        // Create shader and pipeline
        let pipeline =
            self.create_simple_pipeline(shader_source, "LayerNorm Fused", &bind_group_layout);

        // **SINGLE KERNEL LAUNCH** (not 3!)
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "LayerNorm Fused");

        // Copy result to staging
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read result
        let result = self.read_buffer(&staging_buffer, size).await?;

        Ok(result)
    }
}
