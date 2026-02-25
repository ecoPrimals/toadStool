//! GPU compute operations for Masked Select
//!
//! This module contains the GPU execution logic for masked select operation,
//! including prefix sum computation, mask conversion, and the main selection logic.

use super::MaskedSelect;
use crate::device::DeviceCapabilities;
use crate::error::Result;
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const WG: u32 = 256;

/// Compute GPU prefix sum for boolean mask.
/// Returns (prefix_sum_buffer, total_count).
/// Uses exclusive scan: total = scan_out[N-1] + flags_in[N-1].
pub(super) fn compute_prefix_sum_gpu(
    device: &Arc<crate::device::WgpuDevice>,
    mask_buffer: &wgpu::Buffer,
    size: usize,
) -> Result<(wgpu::Buffer, u32)> {
    if size == 0 {
        let empty = device.create_buffer_u32(0)?;
        return Ok((empty, 0));
    }

    let prefix_sum_buffer = device.create_buffer_u32(size)?;
    let n_groups = (size as u32).div_ceil(WG);
    let scratch_buffer = device.create_buffer_u32(n_groups as usize)?;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct ScanConfig {
        n: u32,
        n_groups: u32,
        _pad0: u32,
        _pad1: u32,
    }

    let params = ScanConfig {
        n: size as u32,
        n_groups,
        _pad0: 0,
        _pad1: 0,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PrefixSum Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PrefixSum Bind Group Layout"),
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
                ],
            });

    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("PrefixSum Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: mask_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: prefix_sum_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: scratch_buffer.as_entire_binding(),
            },
        ],
    });

    let shader = device.compile_shader(MaskedSelect::prefix_sum_shader(), Some("PrefixSum"));
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PrefixSum Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let local_scan_pipeline =
        device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("PrefixSum Local Scan Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "local_scan",
                cache: None,
                compilation_options: Default::default(),
            });

    let add_wg_offsets_pipeline =
        device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("PrefixSum Add WG Offsets Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "add_wg_offsets",
                cache: None,
                compilation_options: Default::default(),
            });

    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PrefixSum Encoder"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("PrefixSum Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&local_scan_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(n_groups.max(1), 1, 1);
    }

    if n_groups > 1 {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("PrefixSum Add WG Offsets Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&add_wg_offsets_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    // Total = scan_out[N-1] + flags_in[N-1] (exclusive scan + last flag)
    let scan_last = read_buffer_u32_last(device, &prefix_sum_buffer, size)?;
    let flags_last = read_buffer_u32_last(device, mask_buffer, size)?;
    let total = scan_last + flags_last;

    Ok((prefix_sum_buffer, total))
}

/// Convert f32 mask to u32 mask on GPU
pub(super) fn convert_mask_gpu(
    device: &Arc<crate::device::WgpuDevice>,
    input_mask_buffer: &wgpu::Buffer,
    mask_buffer: &wgpu::Buffer,
    size: usize,
) -> Result<()> {
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct MaskParams {
        size: u32,
        _pad1: u32,
        _pad2: u32,
        _pad3: u32,
    }

    let params = MaskParams {
        size: size as u32,
        _pad1: 0,
        _pad2: 0,
        _pad3: 0,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mask Convert Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Mask Convert Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Mask Convert Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input_mask_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: mask_buffer.as_entire_binding(),
            },
        ],
    });

    let shader = device.compile_shader(MaskedSelect::mask_convert_shader(), Some("Mask Convert"));
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mask Convert Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mask Convert Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
        });

    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mask Convert Encoder"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Mask Convert Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Dispatch using standard 1D shader workgroup size (256)
        let caps = DeviceCapabilities::from_device(device);
        let workgroups = caps.dispatch_1d(size as u32);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    Ok(())
}

/// Read only the last element of a u32 buffer
pub(super) fn read_buffer_u32_last(
    device: &Arc<crate::device::WgpuDevice>,
    buffer: &wgpu::Buffer,
    size: usize,
) -> Result<u32> {
    if size == 0 {
        return Ok(0);
    }
    let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer U32 Last"),
        size: std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Read Buffer Last Encoder"),
        });
    encoder.copy_buffer_to_buffer(
        buffer,
        ((size - 1) * std::mem::size_of::<u32>()) as u64,
        &staging_buffer,
        0,
        std::mem::size_of::<u32>() as u64,
    );
    device.queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) =
        std::sync::mpsc::sync_channel::<std::result::Result<(), wgpu::BufferAsyncError>>(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.device.poll(wgpu::Maintain::Wait);

    receiver
        .recv()
        .map_err(|e| crate::error::BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
        .map_err(|e| crate::error::BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

    let data = buffer_slice.get_mapped_range();
    let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    Ok(result_data[0])
}

/// Execute the masked select operation (GPU compute)
pub(super) fn execute_masked_select(op: MaskedSelect) -> Result<Tensor> {
    let device = op.input().device();
    let input_size: usize = op.input().shape().iter().product();

    // Step 1: Create boolean mask buffer on GPU and convert f32 mask to u32
    let mask_buffer = device.create_buffer_u32(input_size)?;
    convert_mask_gpu(device, op.mask().buffer(), &mask_buffer, input_size)?;

    // Step 2: Compute prefix sum on GPU
    let (prefix_sum_buffer, output_size) =
        compute_prefix_sum_gpu(device, &mask_buffer, input_size)?;
    let output_size = output_size as usize;

    // Handle zero-size output
    if output_size == 0 {
        return Ok(Tensor::new(vec![], vec![0], device.clone()));
    }

    // Access input buffer directly (zero-copy)
    let input_buffer = op.input().buffer();

    // Create output buffer
    let output_buffer = device.create_buffer_f32(output_size)?;

    // Create uniform buffer for parameters
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        input_size: u32,
        _pad1: u32,
        _pad2: u32,
        _pad3: u32,
    }

    let params = Params {
        input_size: input_size as u32,
        _pad1: 0,
        _pad2: 0,
        _pad3: 0,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MaskedSelect Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Compile shader
    let shader_module =
        device.compile_shader(MaskedSelect::wgsl_shader(), Some("MaskedSelect Shader"));

    // Create bind group layout
    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MaskedSelect Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    // Create bind group
    let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MaskedSelect Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: mask_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: prefix_sum_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // Create compute pipeline
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MaskedSelect Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let compute_pipeline =
        device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MaskedSelect Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

    // Execute compute shader
    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MaskedSelect Encoder"),
        });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("MaskedSelect Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        // Dispatch using standard 1D shader workgroup size (256)
        let caps = DeviceCapabilities::from_device(device);
        let workgroups = caps.dispatch_1d(input_size as u32);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    let output_data = crate::utils::read_buffer(device, &output_buffer, output_size)?;
    Ok(Tensor::new(output_data, vec![output_size], device.clone()))
}
