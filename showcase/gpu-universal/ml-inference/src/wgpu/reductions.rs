//! Reduction operations
//!
//! Reduce, DotProduct, and other operations that reduce tensors to scalars.
//! Efficient parallel reduction using GPU workgroups.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::*};

impl WgpuExecutor {
    /// Execute reduce operation: compute sum/max/min/mean
    ///
    /// Deep Debt: Operation type determined at runtime.
    /// Uses efficient parallel reduction with workgroup-local reductions.
    pub async fn execute_reduce(&self, input: &[f32], operation: ReduceOp) -> Result<f32> {
        let size = input.len();
        let workgroups = self.calculate_workgroups(size, 256).max(1);

        let shader_source = include_str!("../shaders/reduce.wgsl");

        let input_buffer = self.create_input_buffer(input, "Reduce Input");

        // Partial results buffer (one per workgroup)
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Results"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reduce Staging"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ReduceParams {
            size: u32,
            operation: u32,
            _padding: [u32; 2],
        }

        let params = ReduceParams {
            size: size as u32,
            operation: operation as u32,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Reduce Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Reduce Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Reduce", &bind_group_layout);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Reduce");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (workgroups as usize * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read partial results and perform final reduction on CPU
        let partial_results = self.read_buffer(&staging_buffer, workgroups as usize).await?;

        // Final reduction on CPU (small array)
        let result = match operation {
            ReduceOp::Sum | ReduceOp::Mean => partial_results.iter().sum::<f32>(),
            ReduceOp::Max => partial_results
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max),
            ReduceOp::Min => partial_results
                .iter()
                .copied()
                .fold(f32::INFINITY, f32::min),
        };

        Ok(if matches!(operation, ReduceOp::Mean) {
            result / size as f32
        } else {
            result
        })
    }

    /// Execute dot product: compute A · B
    ///
    /// Deep Debt: Vector sizes determined at runtime.
    /// Efficient parallel multiply-reduce operation.
    pub async fn execute_dot_product(&self, a: &[f32], b: &[f32]) -> Result<f32> {
        anyhow::ensure!(a.len() == b.len(), "Vectors must have same length for dot product");
        let size = a.len();
        let workgroups = self.calculate_workgroups(size, 256).max(1);

        let shader_source = include_str!("../shaders/dotproduct.wgsl");

        let a_buffer = self.create_input_buffer(a, "DotProduct A");
        let b_buffer = self.create_input_buffer(b, "DotProduct B");

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Sums"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DotProduct Staging"),
            size: (workgroups as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DotProductParams {
            size: u32,
            _padding: [u32; 3],
        }

        let params = DotProductParams {
            size: size as u32,
            _padding: [0; 3],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("DotProduct Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("DotProduct Layout"),
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
            label: Some("DotProduct Bind Group"),
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

        let pipeline =
            self.create_simple_pipeline(shader_source, "DotProduct", &bind_group_layout);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "DotProduct");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (workgroups as usize * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read partial sums and compute final dot product
        let partial_sums = self.read_buffer(&staging_buffer, workgroups as usize).await?;
        Ok(partial_sums.iter().sum())
    }

    /// Execute map operation: apply function to each element
    ///
    /// Deep Debt: Operation type determined at runtime.
    pub async fn execute_map(&self, input: &[f32], operation: MapOp) -> Result<Vec<f32>> {
        let size = input.len();
        let shader_source = include_str!("../shaders/map.wgsl");

        let input_buffer = self.create_input_buffer(input, "Map Input");
        let output_buffer = self.create_output_buffer(size, "Map Output");
        let staging_buffer = self.create_staging_buffer(size, "Map Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MapParams {
            size: u32,
            operation: u32,
            _padding: [u32; 2],
        }

        let params = MapParams {
            size: size as u32,
            operation: operation as u32,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Map Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Map Layout"),
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

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Map Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Map", &bind_group_layout);
        let workgroups = self.calculate_workgroups(size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Map");

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
