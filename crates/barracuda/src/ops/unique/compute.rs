//! GPU compute operations for unique element detection
//!
//! This module contains the multi-pass GPU execution:
//! 1. Pass 1: Mark unique values using hash table (parallel)
//! 2. Pass 2: Compute prefix sum of unique flags (parallel)
//! 3. Pass 3: Read unique count from prefix sum
//! 4. Pass 4: Compact unique values (parallel)

use super::Unique;
use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Compute GPU prefix sum for boolean mask
fn compute_prefix_sum_gpu(
    device: &Arc<crate::device::WgpuDevice>,
    mask_buffer: &wgpu::Buffer,
    size: usize,
) -> Result<wgpu::Buffer> {
    let prefix_sum_buffer = device.create_buffer_u32(size)?;
    let scratch_buffer = device.create_buffer_u32(size)?;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct PrefixSumParams {
        size: u32,
        _pad1: u32,
        _pad2: u32,
        _pad3: u32,
    }

    let params = PrefixSumParams {
        size: size as u32,
        _pad1: 0,
        _pad2: 0,
        _pad3: 0,
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

    let shader = device.compile_shader(Unique::prefix_sum_shader(), Some("PrefixSum"));
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PrefixSum Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PrefixSum Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "inclusive_scan",
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
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Dispatch using standard 1D shader workgroup size (256)
        let caps = DeviceCapabilities::from_device(device);
        let workgroups = caps.dispatch_1d(size as u32);
        pass.dispatch_workgroups(workgroups.max(1), 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    Ok(prefix_sum_buffer)
}

/// Read only the last element of a u32 buffer
fn read_buffer_u32_last(
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
    let (sender, receiver) = futures::channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.device.poll(wgpu::Maintain::Wait);

    futures::executor::block_on(receiver)
        .map_err(|e| BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
        .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

    let data = buffer_slice.get_mapped_range();
    let result_data: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    Ok(result_data[0])
}

/// Execute unique operation
pub(super) fn execute(unique: Unique) -> Result<Tensor> {
    let device = unique.input().device();
    let input_size: usize = unique.input().len();

    // Use hash table size - large minimum to avoid collisions (hash stores only occupancy, not value)
    let num_buckets = (input_size * 32).next_power_of_two().clamp(8192, 65536);

    // Create hash table (atomic u32)
    let hash_table_buffer = device.create_buffer_u32(num_buckets)?;

    // Create unique flags buffer
    let unique_flags_buffer = device.create_buffer_u32(input_size)?;

    // Initialize hash table to zeros
    let zeros = vec![0u32; num_buckets];
    device
        .queue
        .write_buffer(&hash_table_buffer, 0, bytemuck::cast_slice(&zeros));

    // Initialize flags to zeros
    let zeros_flags = vec![0u32; input_size];
    device
        .queue
        .write_buffer(&unique_flags_buffer, 0, bytemuck::cast_slice(&zeros_flags));

    let params = super::UniqueParams {
        input_size: input_size as u32,
        num_buckets: num_buckets as u32,
        _pad1: 0,
        _pad2: 0,
    };

    let params_buffer = device
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unique Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Step 1: Mark unique values
    let bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Unique Bind Group Layout"),
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
        label: Some("Unique Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: unique.input().buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: hash_table_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: unique_flags_buffer.as_entire_binding(),
            },
        ],
    });

    let shader = device.compile_shader(Unique::wgsl_shader(), Some("Unique"));
    let pipeline_layout = device
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Unique Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = device
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Unique Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "mark_unique",
        cache: None,
        compilation_options: Default::default(),
        });

    let mut encoder = device
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Unique Encoder"),
        });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Unique Mark Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Dispatch using standard 1D shader workgroup size (256)
        let caps = DeviceCapabilities::from_device(device);
        let workgroups = caps.dispatch_1d(input_size as u32);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.queue.submit(Some(encoder.finish()));

    // Step 2: Compute prefix sum of unique flags to determine output positions
    let prefix_sum_buffer = compute_prefix_sum_gpu(device, &unique_flags_buffer, input_size)?;

    // Step 3: Read only the last element to get unique count
    let unique_count = read_buffer_u32_last(device, &prefix_sum_buffer, input_size)? as usize;

    if unique_count == 0 {
        return Ok(Tensor::new(vec![], vec![0], device.clone()));
    }

    // Step 4: Compact unique values using GPU shader
    let output_buffer = device.create_buffer_f32(unique_count)?;

    // Update bind group for compaction pass
    let compact_bind_group_layout =
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Unique Compact Bind Group Layout"),
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
                            // Must match WGSL: hash_table is atomic<u32> (read_write)
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
                            // Must match WGSL: unique_flags is read_write
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

    let compact_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Unique Compact Bind Group"),
        layout: &compact_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: unique.input().buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: hash_table_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: unique_flags_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: prefix_sum_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    let compact_pipeline_layout =
        device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Unique Compact Pipeline Layout"),
                bind_group_layouts: &[&compact_bind_group_layout],
                push_constant_ranges: &[],
            });

    let compact_pipeline =
        device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Unique Compact Pipeline"),
                layout: Some(&compact_pipeline_layout),
                module: &shader,
                entry_point: "compact_unique",
            cache: None,
            compilation_options: Default::default(),
            });

    let mut compact_encoder =
        device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Unique Compact Encoder"),
            });

    {
        let mut pass = compact_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Unique Compact Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&compact_pipeline);
        pass.set_bind_group(0, &compact_bind_group, &[]);
        // Dispatch using standard 1D shader workgroup size (256)
        let caps = DeviceCapabilities::from_device(device);
        let workgroups = caps.dispatch_1d(input_size as u32);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    device.queue.submit(Some(compact_encoder.finish()));

    let output_data = crate::utils::read_buffer(device, &output_buffer, unique_count)?;
    Ok(Tensor::new(output_data, vec![unique_count], device.clone()))
}
