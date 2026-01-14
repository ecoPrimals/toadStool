//! Regularization operations
//!
//! Dropout and other regularization techniques for training.
//! Prevents overfitting and improves generalization.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::executor::WgpuExecutor;

impl WgpuExecutor {
    /// Execute dropout: regularization with random masking
    ///
    /// During training: Randomly zeros elements with probability `dropout_prob`
    /// During inference: Returns input unchanged (training=false)
    ///
    /// Deep Debt: Training flag and seed determined at runtime, not compile-time.
    pub async fn execute_dropout(
        &self,
        input: &[f32],
        dropout_prob: f32,
        training: bool,
        seed: Option<u64>,
    ) -> Result<Vec<f32>> {
        let size = input.len();

        // If not training, just return input (no dropout)
        if !training {
            return Ok(input.to_vec());
        }

        let shader_source = include_str!("../shaders/dropout.wgsl");

        let input_buffer = self.create_input_buffer(input, "Dropout Input");
        let output_buffer = self.create_output_buffer(size, "Dropout Output");
        let staging_buffer = self.create_staging_buffer(size, "Dropout Staging");

        // Mask buffer for random dropout pattern
        let mask_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dropout Mask"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DropoutParams {
            size: u32,
            dropout_prob: f32,
            training: u32,
            seed: u32,
        }

        let params = DropoutParams {
            size: size as u32,
            dropout_prob,
            training: if training { 1 } else { 0 },
            seed: seed.unwrap_or_else(|| {
                // Generate seed from current time if not provided (Deep Debt: runtime!)
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }) as u32,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dropout Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Dropout Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dropout Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "Dropout", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Dropout");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }
}
