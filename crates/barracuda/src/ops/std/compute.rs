//! GPU compute operations for Standard Deviation
//!
//! This module contains the GPU execution logic for standard deviation reduction,
//! supporting both global reduction and dimension-wise reduction.

use super::Std;
use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

pub(super) fn execute(op: Std) -> Result<Tensor> {
    let device = op.input.device();
    let shape = op.input.shape();
    let input_buffer = op.input.buffer();

    match op.dim {
        None => {
            // Global std reduction
            // Two-pass algorithm: first compute mean, then variance, then sqrt
            let size: usize = shape.iter().product();
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let num_workgroups = (size as u32).div_ceil(optimal_wg_size);

            // Pass 1: Compute mean using tree reduction
            let mean_output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Std Mean Output"),
                size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct Params {
                size: u32,
            }

            let params = Params { size: size as u32 };

            let params_buffer =
                device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Std Mean Params"),
                        contents: bytemuck::cast_slice(&[params]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            let shader_module =
                device.compile_shader(Std::wgsl_shader_reduce(), Some("Std Reduce Shader"));

            let bind_group_layout =
                device
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Std Reduce Bind Group Layout"),
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
                label: Some("Std Mean Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: mean_output_buffer.as_entire_binding(),
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
                        label: Some("Std Reduce Pipeline Layout"),
                        bind_group_layouts: &[&bind_group_layout],
                        push_constant_ranges: &[],
                    });

            let compute_pipeline =
                device
                    .device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Std Reduce Pipeline"),
                        layout: Some(&pipeline_layout),
                        module: &shader_module,
                        entry_point: "main",
                        cache: None,
                        compilation_options: Default::default(),
                    });

            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Std Reduce Encoder"),
                    });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Std Mean Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }

            device.submit_and_poll(Some(encoder.finish()));

            // Read back partial sums and compute mean
            let partial_sums =
                device.read_buffer_f32(&mean_output_buffer, num_workgroups as usize)?;
            let global_sum: f32 = partial_sums.iter().sum();
            let global_mean = global_sum / size as f32;

            // Pass 2: Compute variance using tree reduction with mean
            // Create a buffer with (x - mean)^2 values
            let diff_squared_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Std Diff Squared"),
                size: (size * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Compute (x - mean)^2 on CPU for now
            // In a more optimized version, this could be done on GPU
            let input_data = device.read_buffer_f32(input_buffer, size)?;
            let diff_squared: Vec<f32> = input_data
                .iter()
                .map(|&x| {
                    let diff = x - global_mean;
                    diff * diff
                })
                .collect();

            device
                .queue
                .write_buffer(&diff_squared_buffer, 0, bytemuck::cast_slice(&diff_squared));

            // Now reduce the diff_squared buffer
            let variance_output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Std Output"),
                size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let variance_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Std Variance Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: diff_squared_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: variance_output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder2 =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Std Encoder 2"),
                    });

            {
                let mut compute_pass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Std Variance Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&compute_pipeline);
                compute_pass.set_bind_group(0, &variance_bind_group, &[]);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }

            device.submit_and_poll(Some(encoder2.finish()));

            // Read back partial variance results
            let partial_variances =
                device.read_buffer_f32(&variance_output_buffer, num_workgroups as usize)?;
            let global_variance_sum: f32 = partial_variances.iter().sum();
            let global_variance = global_variance_sum / size as f32;
            let global_std = global_variance.sqrt();

            // Return scalar tensor
            Ok(Tensor::new(vec![global_std], vec![], device.clone()))
        }
        Some(dim) => {
            // Dimension-wise std reduction
            if dim >= shape.len() {
                return Err(crate::error::BarracudaError::InvalidInput {
                    message: format!("Dimension {} out of range for shape {:?}", dim, shape),
                });
            }

            let dim_size = shape[dim];
            let outer_size: usize = shape[..dim].iter().product();
            let inner_size: usize = shape[dim + 1..].iter().product();
            let output_size = outer_size * inner_size;

            // Create output buffer
            let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Std Dim Output"),
                size: (output_size * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Create uniform buffer for parameters
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct Params {
                dim_size: u32,
                outer_size: u32,
                inner_size: u32,
            }

            let params = Params {
                dim_size: dim_size as u32,
                outer_size: outer_size as u32,
                inner_size: inner_size as u32,
            };

            let params_buffer =
                device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Std Dim Params"),
                        contents: bytemuck::cast_slice(&[params]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            // Compile shader
            let shader_module =
                device.compile_shader(Std::wgsl_shader_dim(), Some("Std Dim Shader"));

            // Create bind group layout
            let bind_group_layout =
                device
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Std Dim Bind Group Layout"),
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

            // Create bind group
            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Std Dim Bind Group"),
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
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            // Create compute pipeline
            let pipeline_layout =
                device
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Std Dim Pipeline Layout"),
                        bind_group_layouts: &[&bind_group_layout],
                        push_constant_ranges: &[],
                    });

            let compute_pipeline =
                device
                    .device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Std Dim Pipeline"),
                        layout: Some(&pipeline_layout),
                        module: &shader_module,
                        entry_point: "main",
                        cache: None,
                        compilation_options: Default::default(),
                    });

            // Execute compute shader
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Std Dim Encoder"),
                    });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Std Dim Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                // Deep Debt Evolution: Capability-based dispatch
                let caps = DeviceCapabilities::from_device(device);
                let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
                let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            device.submit_and_poll(Some(encoder.finish()));

            // Read back results
            let output_data = device.read_buffer_f32(&output_buffer, output_size)?;

            // Calculate output shape
            let mut output_shape = shape.to_vec();
            if op.keepdim {
                output_shape[dim] = 1;
            } else {
                output_shape.remove(dim);
            }

            Ok(Tensor::new(output_data, output_shape, device.clone()))
        }
    }
}
