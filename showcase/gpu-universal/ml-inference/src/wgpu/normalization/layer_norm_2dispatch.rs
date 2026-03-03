// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layer Normalization - 2-Dispatch variant
//!

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    pub async fn execute_layernorm_2dispatch(
        &self,
        input: &[f32],
        config: NormConfig,
    ) -> Result<Vec<f32>> {
        let size = input.len();
        anyhow::ensure!(size > 0, "LayerNorm 2-Dispatch: input cannot be empty");

        // Single workgroup for statistics computation (simpler, works correctly)
        let stats_workgroups = 1u32;

        // ═══════════════════════════════════════════════════════════
        // DISPATCH 1: Compute Mean + Variance (Single Pass)
        // ═══════════════════════════════════════════════════════════

        let meanvar_shader = include_str!("../../shaders/layernorm_meanvar.wgsl");

        let input_buffer = self.create_input_buffer(input, "LayerNorm 2D Input");

        // Stats buffer: [mean, variance]
        let stats_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerNorm 2D Stats"),
            size: (2 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MeanVarParams {
            size: u32,
            epsilon: f32,
        }

        let meanvar_params = MeanVarParams {
            size: size as u32,
            epsilon: config.epsilon,
        };

        let meanvar_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("LayerNorm 2D MeanVar Params"),
                    contents: bytemuck::bytes_of(&meanvar_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let meanvar_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LayerNorm 2D MeanVar Layout"),
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

        let meanvar_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LayerNorm 2D MeanVar Bind Group"),
            layout: &meanvar_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: meanvar_params_buffer.as_entire_binding(),
                },
            ],
        });

        let meanvar_pipeline = self.create_simple_pipeline(
            meanvar_shader,
            "LayerNorm 2D MeanVar",
            &meanvar_bind_group_layout,
        );

        // Execute dispatch 1: Compute mean + variance
        let encoder = self.execute_compute_pass(
            &meanvar_pipeline,
            &meanvar_bind_group,
            stats_workgroups,
            "LayerNorm 2D MeanVar Pass",
        );

        self.queue.submit(Some(encoder.finish()));

        // ═══════════════════════════════════════════════════════════
        // DISPATCH 2: Normalize with Statistics
        // ═══════════════════════════════════════════════════════════

        let normalize_shader = include_str!("../../shaders/layernorm_normalize.wgsl");
        let normalize_workgroups = self.calculate_workgroups(size, 256).max(1).min(65535);

        let gamma = config.gamma.unwrap_or_else(|| vec![1.0; size]);
        anyhow::ensure!(
            gamma.len() == size,
            "LayerNorm 2D: gamma size must match input size"
        );
        let gamma_buffer = self.create_input_buffer(&gamma, "LayerNorm 2D Gamma");

        let beta = config.beta.unwrap_or_else(|| vec![0.0; size]);
        anyhow::ensure!(
            beta.len() == size,
            "LayerNorm 2D: beta size must match input size"
        );
        let beta_buffer = self.create_input_buffer(&beta, "LayerNorm 2D Beta");

        let output_buffer = self.create_output_buffer(size, "LayerNorm 2D Output");
        let staging_buffer = self.create_staging_buffer(size, "LayerNorm 2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct NormalizeParams {
            size: u32,
            epsilon: f32,
        }

        let normalize_params = NormalizeParams {
            size: size as u32,
            epsilon: config.epsilon,
        };

        let normalize_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("LayerNorm 2D Normalize Params"),
                    contents: bytemuck::bytes_of(&normalize_params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let normalize_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LayerNorm 2D Normalize Layout"),
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

        let normalize_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LayerNorm 2D Normalize Bind Group"),
            layout: &normalize_bind_group_layout,
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
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: normalize_params_buffer.as_entire_binding(),
                },
            ],
        });

        let normalize_pipeline = self.create_simple_pipeline(
            normalize_shader,
            "LayerNorm 2D Normalize",
            &normalize_bind_group_layout,
        );

        // Execute dispatch 2: Normalize
        let mut encoder = self.execute_compute_pass(
            &normalize_pipeline,
            &normalize_bind_group,
            normalize_workgroups,
            "LayerNorm 2D Normalize Pass",
        );

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
