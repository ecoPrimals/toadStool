// SPDX-License-Identifier: AGPL-3.0-or-later
//! Softmax activation
//!

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::executor::WgpuExecutor;

impl WgpuExecutor {
    /// Execute softmax: stable softmax activation (full GPU multi-pass)
    ///
    /// Implementation: Three-pass GPU pipeline for numerical stability
    /// Pass 1: Find max (GPU reduction)
    /// Pass 2: Compute exp(x - max) and sum (GPU)
    /// Pass 3: Normalize (divide by sum, GPU)
    ///
    /// Deep Debt: No hardcoded sizes, all runtime-configured.
    pub async fn execute_softmax(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = input.len();
        let workgroups = self.calculate_workgroups(size, 256).max(1);

        let shader_source = include_str!("../../shaders/softmax.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Softmax Input");
        let output_buffer = self.create_output_buffer(size, "Softmax Output");
        let staging_buffer = self.create_staging_buffer(size, "Softmax Staging");

        // Intermediate buffers for multi-pass algorithm
        let max_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Max Values"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sum_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sum Values"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SoftmaxParams {
            size: u32,
            _padding: [u32; 3], // Align to 16 bytes
        }

        let params = SoftmaxParams {
            size: size as u32,
            _padding: [0; 3],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Softmax Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout (5 bindings for multi-pass)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Softmax Layout"),
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
            label: Some("Softmax Bind Group"),
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
                    resource: max_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipelines for each pass
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Softmax Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Softmax Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Three pipelines for three passes
        let find_max_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Find Max"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "find_max",
                    compilation_options: Default::default(),
                    cache: None,
                });

        let exp_sum_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Exp Sum"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "compute_exp_sum",
                    compilation_options: Default::default(),
                    cache: None,
                });

        let normalize_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Softmax Normalize"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "normalize",
                    compilation_options: Default::default(),
                    cache: None,
                });

        // Execute three-pass algorithm
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Softmax Encoder"),
            });

        {
            // Pass 1: Find max
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Find Max Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&find_max_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            // Pass 2: Compute exp and sum
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Exp Sum Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&exp_sum_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        {
            // Pass 3: Normalize
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Normalize Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&normalize_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(input) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }
}
