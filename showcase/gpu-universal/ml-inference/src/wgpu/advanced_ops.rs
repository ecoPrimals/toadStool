//! Advanced tensor operations
//!
//! Gather, Scatter, Scan and other advanced indexing/reduction operations.
//! Essential for sparse operations, embeddings, and parallel algorithms.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::ScanOp};

impl WgpuExecutor {
    /// Execute Gather: Index-based element collection
    ///
    /// Gathers elements from source array using indices.
    /// Essential for embedding lookups and sparse access patterns.
    ///
    /// Deep Debt: Indices determined at runtime, no fixed access patterns.
    pub async fn execute_gather(
        &self,
        source: &[f32],
        indices: &[u32],
    ) -> Result<Vec<f32>> {
        let num_elements = indices.len();
        let source_size = source.len();

        let shader_source = include_str!("../shaders/gather.wgsl");

        let source_buffer = self.create_input_buffer(source, "Gather Source");
        let indices_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gather Indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output_buffer = self.create_output_buffer(num_elements, "Gather Output");
        let staging_buffer = self.create_staging_buffer(num_elements, "Gather Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GatherParams {
            num_elements: u32,
            source_size: u32,
            _padding: [u32; 2],
        }

        let params = GatherParams {
            num_elements: num_elements as u32,
            source_size: source_size as u32,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gather Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Custom 4-binding layout for Gather (source, indices, output, params)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Gather Layout"),
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
            label: Some("Gather Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buffer.as_entire_binding(),
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
            self.create_simple_pipeline(shader_source, "Gather", &bind_group_layout);
        let workgroups = self.calculate_workgroups(num_elements, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Gather");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (num_elements * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, num_elements).await
    }

    /// Execute Scatter: Write values to indexed positions
    ///
    /// Scatters source values to destination using indices.
    /// Uses atomic operations for safe concurrent writes.
    ///
    /// Deep Debt: Atomic safety without hardcoded synchronization.
    pub async fn execute_scatter(
        &self,
        source: &[f32],
        indices: &[u32],
        dest_size: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            source.len() == indices.len(),
            "Scatter: source length must equal indices length"
        );

        let num_elements = source.len();
        let shader_source = include_str!("../shaders/scatter.wgsl");

        let source_buffer = self.create_input_buffer(source, "Scatter Source");
        let indices_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scatter Indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Destination buffer: initialize with zeros (i32 for atomic operations)
        let dest_zeros: Vec<i32> = vec![0i32; dest_size];
        let dest_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scatter Dest"),
                    contents: bytemuck::cast_slice(&dest_zeros),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scatter Staging"),
            size: (dest_size * std::mem::size_of::<i32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ScatterParams {
            num_elements: u32,
            dest_size: u32,
            _padding: [u32; 2],
        }

        let params = ScatterParams {
            num_elements: num_elements as u32,
            dest_size: dest_size as u32,
            _padding: [0; 2],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scatter Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        // Custom 4-binding layout for Scatter (source, indices, dest, params)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Scatter Layout"),
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
            label: Some("Scatter Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dest_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "Scatter", &bind_group_layout);
        let workgroups = self.calculate_workgroups(num_elements, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Scatter");

        encoder.copy_buffer_to_buffer(
            &dest_buffer,
            0,
            &staging_buffer,
            0,
            (dest_size * std::mem::size_of::<i32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read i32 results and convert to f32
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        self.device.poll(wgpu::Maintain::Wait);

        receiver
            .receive()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive buffer mapping result"))?
            .context("Failed to map staging buffer")?;

        let data = buffer_slice.get_mapped_range();
        let i32_values: &[i32] = bytemuck::cast_slice(&data);
        // Reinterpret i32 bits as f32 (not cast - scatter stores f32 bit patterns as i32 for atomics)
        let result: Vec<f32> = i32_values.iter().map(|&x| f32::from_bits(x as u32)).collect();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Execute Scan: Parallel prefix sum (and other operations)
    ///
    /// Computes cumulative operation across array elements.
    /// Supports inclusive and exclusive scans.
    ///
    /// NOTE: Currently supports up to 512 elements (single workgroup).
    /// Future: Hierarchical scan for larger arrays.
    ///
    /// Deep Debt: Operation type and scan mode determined at runtime.
    pub async fn execute_scan(
        &self,
        input: &[f32],
        operation: ScanOp,
        exclusive: bool,
    ) -> Result<Vec<f32>> {
        let size = input.len();

        // Validate size (Deep Debt: graceful limit, not hardcoded crash)
        anyhow::ensure!(
            size <= 512,
            "Scan currently supports up to 512 elements (got {}). \
             Hierarchical scan for larger arrays coming soon!",
            size
        );

        let shader_source = include_str!("../shaders/scan.wgsl");

        let input_buffer = self.create_input_buffer(input, "Scan Input");
        let output_buffer = self.create_output_buffer(size, "Scan Output");
        let staging_buffer = self.create_staging_buffer(size, "Scan Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ScanParams {
            size: u32,
            operation: u32,
            exclusive: u32,
            _padding: u32,
        }

        let params = ScanParams {
            size: size as u32,
            operation: operation as u32,
            exclusive: if exclusive { 1 } else { 0 },
            _padding: 0,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scan Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Scan Layout"),
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
            label: Some("Scan Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Scan", &bind_group_layout);

        // Single workgroup for up to 512 elements
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Scan Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Scan Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1); // Single workgroup
        }

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
