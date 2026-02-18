//! GPU compute operations for Expand
//!
//! This module contains broadcasting shape computation, stride calculation,
//! and GPU pipeline execution for the expand operation.

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Compute broadcasted input shape following NumPy broadcasting rules
///
/// Broadcasting rules:
/// - Dimensions are compared right-to-left
/// - Each dimension must either:
///   - Be equal
///   - One of them is 1
///   - One of them doesn't exist (implicitly 1)
/// - For expand operation:
///   - If target_rank > input_rank: pad dimensions at the back (right) with 1s
///   - If target_rank == input_rank: try padding at front first, then validate
pub fn compute_broadcast_shape(
    input_shape: &[usize],
    target_shape: &[usize],
) -> Result<Vec<usize>> {
    let input_rank = input_shape.len();
    let target_rank = target_shape.len();

    let mut broadcasted_input_shape = vec![1; target_rank];

    if target_rank > input_rank {
        // Pad at the back (right): [3] → [3, 1] for target [3, 5]
        for (i, &dim) in input_shape.iter().enumerate() {
            broadcasted_input_shape[i] = dim;
        }
    } else if target_rank == input_rank {
        // Same rank: check if dimensions are compatible
        // For [3] → [9]: this will be handled specially in execute_expand
        // For validation here, we allow it if target is a multiple of input (1D case)
        let mut compatible = true;
        for i in (0..target_rank).rev() {
            let input_dim = input_shape[i];
            let target_dim = target_shape[i];
            if input_dim != target_dim && input_dim != 1 && target_dim != 1 {
                // Special case: if this is the last (only) dimension and target is a multiple of input
                // This will be handled specially in execute_expand by reshaping
                if i == target_rank - 1 && target_rank == 1 && target_dim.is_multiple_of(input_dim)
                {
                    // Allow it - will be handled in execute_expand
                    compatible = true;
                    break;
                } else {
                    compatible = false;
                    break;
                }
            }
        }

        if compatible {
            broadcasted_input_shape = input_shape.to_vec();
        } else {
            // Try back-padding
            for (i, &dim) in input_shape.iter().enumerate() {
                broadcasted_input_shape[i] = dim;
            }
        }
    } else {
        // target_rank < input_rank: shouldn't happen for expand, but handle it
        let offset = target_rank.saturating_sub(input_rank);
        for (i, &dim) in input_shape.iter().enumerate() {
            if offset + i < target_rank {
                broadcasted_input_shape[offset + i] = dim;
            }
        }
    }

    // Validate broadcasting compatibility (right-to-left)
    for i in (0..target_rank).rev() {
        let input_dim = broadcasted_input_shape[i];
        let target_dim = target_shape[i];

        if input_dim != target_dim && input_dim != 1 && target_dim != 1 {
            return Err(BarracudaError::InvalidShape {
                expected: target_shape.to_vec(),
                actual: input_shape.to_vec(),
            });
        }
    }

    Ok(broadcasted_input_shape)
}

/// Compute input strides for broadcasting
///
/// For broadcasting: if shape[i] == 1, stride[i] = 0
fn compute_input_strides(broadcasted_input_shape: &[usize], num_dims: usize) -> Vec<usize> {
    let mut input_strides = vec![0; num_dims];

    // Compute strides backwards
    // Start with last dimension
    if broadcasted_input_shape[num_dims - 1] != 1 {
        input_strides[num_dims - 1] = 1;
    }

    // For each dimension, compute stride as product of subsequent dimensions
    for i in (0..num_dims - 1).rev() {
        if broadcasted_input_shape[i] == 1 {
            // Broadcast dimension: stride is 0
            input_strides[i] = 0;
        } else {
            // Compute stride as product of all dimensions after this one
            let mut stride = 1u32;
            for j in (i + 1)..num_dims {
                stride *= broadcasted_input_shape[j] as u32;
            }
            input_strides[i] = stride as usize;
        }
    }

    input_strides
}

/// Compute output strides
fn compute_output_strides(target_shape: &[usize], num_dims: usize) -> Vec<usize> {
    let mut output_strides = vec![1; num_dims];
    for i in (0..num_dims - 1).rev() {
        output_strides[i] = output_strides[i + 1] * target_shape[i + 1];
    }
    output_strides
}

/// Execute expand operation on GPU
pub fn execute_expand(input: Tensor, target_shape: Vec<usize>) -> Result<Tensor> {
    let device = input.device();
    let input_shape = input.shape();
    let output_size: usize = target_shape.iter().product();
    let original_target_shape = target_shape.clone();

    // Special case: handle [3] → [9] type expansions where ranks are equal
    // but target size is a multiple of input size
    // We treat this as adding a leading dimension: [3] → [1, 3] → [3, 3] → [9]
    let (effective_target_shape, effective_broadcasted_input_shape) = if input_shape.len()
        == original_target_shape.len()
        && input_shape.len() == 1
        && original_target_shape[0].is_multiple_of(input_shape[0])
        && original_target_shape[0] != input_shape[0]
    {
        // [3] → [9]: treat as [1, 3] → [3, 3]
        let leading_dim = original_target_shape[0] / input_shape[0];
        let effective_target = vec![leading_dim, input_shape[0]];
        let effective_input = vec![1, input_shape[0]];
        (effective_target, effective_input)
    } else {
        // Normal case: use target_shape as-is
        let broadcasted = compute_broadcast_shape(input_shape, &original_target_shape)?;
        (original_target_shape.clone(), broadcasted)
    };

    let broadcasted_input_shape = effective_broadcasted_input_shape;
    let target_shape = effective_target_shape;

    // Compute strides
    let num_dims = target_shape.len();
    let input_strides = compute_input_strides(&broadcasted_input_shape, num_dims);
    let output_strides = compute_output_strides(&target_shape, num_dims);

    let output_buffer = device.create_buffer_f32(output_size)?;

    // Create buffers for shapes and strides
    let input_shape_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Input Shape"),
            contents: bytemuck::cast_slice(
                &broadcasted_input_shape
                    .iter()
                    .map(|&x| x as u32)
                    .collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    let output_shape_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Output Shape"),
            contents: bytemuck::cast_slice(
                &target_shape.iter().map(|&x| x as u32).collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    let input_strides_buffer =
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Expand Input Strides"),
                contents: bytemuck::cast_slice(
                    &input_strides.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                ),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

    let output_strides_buffer =
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Expand Output Strides"),
                contents: bytemuck::cast_slice(
                    &output_strides.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                ),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

    // Create params buffer
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        output_size: u32,
        num_dims: u32,
        _pad1: u32,
        _pad2: u32,
    }

    let params = Params {
        output_size: output_size as u32,
        num_dims: num_dims as u32,
        _pad1: 0,
        _pad2: 0,
    };
    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Expand Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Bind group layout (7 bindings: params, input, input_shape, output_shape, input_strides, output_strides, output)
    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Expand BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Expand BG"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: input_shape_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: output_shape_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: input_strides_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output_strides_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    let shader = device.compile_shader(
        include_str!("../../shaders/math/expand.wgsl"),
        Some("Expand"),
    );
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Expand PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Expand Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
        });

    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Expand Encoder"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Expand Pass"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);

        // Deep Debt Evolution: Capability-based dispatch
        let caps = DeviceCapabilities::from_device(device);
        let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
        let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    Ok(Tensor::from_buffer(
        output_buffer,
        original_target_shape,
        device.clone(),
    ))
}
