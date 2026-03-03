// SPDX-License-Identifier: AGPL-3.0-or-later
//! Arithmetic and element-wise binary operations
//!
//! Add, Sub, Mul, Div - SAXPY and element-wise ops.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::{executor::WgpuExecutor, types::BinaryOp};

impl WgpuExecutor {
    pub async fn execute_add(&self, a: &[f32], b: &[f32], alpha: f32) -> Result<Vec<f32>> {
        assert_eq!(a.len(), b.len(), "Vectors must be same length");
        let size = a.len();

        let shader_source = include_str!("../../shaders/add.wgsl");

        let a_buffer = self.create_input_buffer(a, "Add A");
        let b_buffer = self.create_input_buffer(b, "Add B");
        let c_buffer = self.create_output_buffer(size, "Add C");
        let staging_buffer = self.create_staging_buffer(size, "Add Staging");

        // Runtime parameter (not hardcoded!)
        let params = [alpha, 0.0, 0.0, 0.0]; // Pad to 16 bytes
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Add Params"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Similar pattern but with params binding...
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Add Bind Group Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Add Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: c_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Add", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Add");

        encoder.copy_buffer_to_buffer(
            &c_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(a) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute elementwise binary operation: C = A op B
    ///
    /// Deep Debt: Operation type determined at runtime, not compile-time.
    pub async fn execute_elementwise_binary(
        &self,
        a: &[f32],
        b: &[f32],
        operation: BinaryOp,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            a.len() == b.len(),
            "Vector sizes must match for binary operation"
        );
        let size = a.len();

        let shader_source = include_str!("../../shaders/elementwise_binary.wgsl");

        let a_buffer = self.create_input_buffer(a, "Binary A");
        let b_buffer = self.create_input_buffer(b, "Binary B");
        let output_buffer = self.create_output_buffer(size, "Binary Output");
        let staging_buffer = self.create_staging_buffer(size, "Binary Staging");

        // Runtime parameters (Deep Debt: operation determined at runtime)
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BinaryParams {
            size: u32,
            operation: u32,
        }

        let params = BinaryParams {
            size: size as u32,
            operation: operation as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Binary Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout (3 inputs + params)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Binary Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binary Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buffer.as_entire_binding(),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Binary", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Binary");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of_val(a) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }
}
