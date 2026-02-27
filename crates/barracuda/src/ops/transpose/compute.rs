//! GPU compute operations for Transpose
//!
//! This module contains GPU pipeline setup, buffer creation, and execution
//! logic for both 2D and N-D transpose operations.

use super::TransposeParams2D;
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Execute transpose operation
pub fn execute_transpose(input: Tensor, permutation: Option<Vec<usize>>) -> Result<Tensor> {
    let device = input.device().clone();
    let shape = input.shape().to_vec();
    let num_dims = shape.len();
    let size = input.len();

    // Determine if 2D or N-D
    let is_2d = num_dims == 2 && permutation.is_none();

    if is_2d {
        // Optimized 2D transpose
        execute_2d(input, &device, &shape, size)
    } else {
        // N-D transpose with permutation
        let perm = permutation.unwrap_or_else(|| {
            // Default: swap last two dimensions
            let mut p: Vec<usize> = (0..num_dims).collect();
            if num_dims >= 2 {
                p.swap(num_dims - 2, num_dims - 1);
            }
            p
        });
        execute_nd(input, &device, &shape, size, perm)
    }
}

fn execute_2d(
    input: Tensor,
    device: &std::sync::Arc<crate::device::WgpuDevice>,
    shape: &[usize],
    size: usize,
) -> Result<Tensor> {
    let rows = shape[0] as u32;
    let cols = shape[1] as u32;

    // Create output buffer
    let output_buffer = device.create_buffer_f32(size)?;

    // Create params buffer
    let params_2d = TransposeParams2D {
        rows,
        cols,
        _padding: [0, 0],
    };
    let params_bytes = bytemuck::bytes_of(&params_2d);
    let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Transpose Params 2D"),
        size: params_bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    device.queue.write_buffer(&params_buffer, 0, params_bytes);

    // Create bind group layout
    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transpose Bind Group Layout 2D"),
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

    // Create bind group
    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Transpose Bind Group 2D"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    // Compile shader
    let shader = device.compile_shader(super::Transpose::wgsl_shader(), Some("Transpose 2D"));

    // Create pipeline
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transpose Pipeline Layout 2D"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Transpose Pipeline 2D"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main_2d",
            cache: None,
            compilation_options: Default::default(),
        });

    // Encode and execute
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Transpose Encoder 2D"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Transpose Pass 2D"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);

        // S-16: 2D tiled transpose uses @workgroup_size(16, 16) in the shader.
        // Each workgroup covers a 16x16 tile, so dispatch in tiles not elements.
        const TILE: u32 = 16;
        let workgroups_x = cols.div_ceil(TILE).max(1);
        let workgroups_y = rows.div_ceil(TILE).max(1);
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    device.submit_and_poll(Some(encoder.finish()));

    // Create output tensor with transposed shape
    let new_shape = vec![shape[1], shape[0]];
    Ok(Tensor::from_buffer(
        output_buffer,
        new_shape,
        device.clone(),
    ))
}

fn execute_nd(
    input: Tensor,
    device: &std::sync::Arc<crate::device::WgpuDevice>,
    shape: &[usize],
    size: usize,
    permutation: Vec<usize>,
) -> Result<Tensor> {
    let num_dims = shape.len();

    // Compute output shape
    let output_shape: Vec<usize> = permutation.iter().map(|&idx| shape[idx]).collect();

    // Compute input strides
    let mut input_strides = vec![1; num_dims];
    for i in (0..num_dims - 1).rev() {
        input_strides[i] = input_strides[i + 1] * shape[i + 1];
    }

    // Compute output strides
    let mut output_strides = vec![1; num_dims];
    for i in (0..num_dims - 1).rev() {
        output_strides[i] = output_strides[i + 1] * output_shape[i + 1];
    }

    // Create output buffer
    let output_buffer = device.create_buffer_f32(size)?;

    // Create buffers for shape and stride data
    let input_shape_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Input Shape"),
            contents: bytemuck::cast_slice(&shape.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    let output_shape_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Output Shape"),
            contents: bytemuck::cast_slice(
                &output_shape.iter().map(|&x| x as u32).collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    let permutation_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Permutation"),
            contents: bytemuck::cast_slice(
                &permutation.iter().map(|&x| x as u32).collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    let input_strides_buffer =
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Transpose Input Strides"),
                contents: bytemuck::cast_slice(
                    &input_strides.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                ),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

    let output_strides_buffer =
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Transpose Output Strides"),
                contents: bytemuck::cast_slice(
                    &output_strides.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                ),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

    // Create params
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        total_size: u32,
        num_dims: u32,
        is_2d: u32,
        _padding: u32,
    }

    let params = Params {
        total_size: size as u32,
        num_dims: num_dims as u32,
        is_2d: 0,
        _padding: 0,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Create bind group layout
    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transpose Bind Group Layout ND"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    // Create bind group
    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Transpose Bind Group ND"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: input_shape_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output_shape_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: permutation_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: input_strides_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: output_strides_buffer.as_entire_binding(),
            },
        ],
    });

    // Compile shader
    let shader = device.compile_shader(super::Transpose::wgsl_shader(), Some("Transpose ND"));

    // Create pipeline
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transpose Pipeline Layout ND"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Transpose Pipeline ND"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main_nd",
            cache: None,
            compilation_options: Default::default(),
        });

    // Encode and execute
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Transpose Encoder ND"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Transpose Pass ND"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);

        // Deep Debt Evolution: Capability-based dispatch
        use crate::device::{DeviceCapabilities, WorkloadType};
        let caps = DeviceCapabilities::from_device(device);
        let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
        let workgroups = (size as u32).div_ceil(optimal_wg_size);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.submit_and_poll(Some(encoder.finish()));

    // Create output tensor with transposed shape
    Ok(Tensor::from_buffer(
        output_buffer,
        output_shape,
        std::sync::Arc::clone(device),
    ))
}
