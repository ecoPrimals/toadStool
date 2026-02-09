//! Data manipulation operations
//!
//! Tensor operations for data manipulation: concat, slice, pad, reshape, etc.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::executor::WgpuExecutor;

impl WgpuExecutor {
    /// Execute Concat: Concatenate two tensors along axis 0
    ///
    /// Joins two tensors along the first dimension (batch or channel).
    /// Essential for skip connections, feature fusion, multi-path networks.
    ///
    /// Deep Debt: Dimensions determined at runtime, flexible concatenation.
    ///
    /// Use cases: ResNet skip connections, U-Net feature fusion, DenseNet.
    pub async fn execute_concat(&self, input1: &[f32], input2: &[f32]) -> Result<Vec<f32>> {
        anyhow::ensure!(!input1.is_empty(), "Concat: input1 cannot be empty");
        anyhow::ensure!(!input2.is_empty(), "Concat: input2 cannot be empty");

        let size1 = input1.len();
        let size2 = input2.len();
        let output_size = size1 + size2;

        let shader_source = include_str!("../shaders/concat.wgsl");

        // Create buffers
        let input1_buffer = self.create_input_buffer(input1, "Concat Input1");
        let input2_buffer = self.create_input_buffer(input2, "Concat Input2");
        let output_buffer = self.create_output_buffer(output_size, "Concat Output");
        let staging_buffer = self.create_staging_buffer(output_size, "Concat Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ConcatParams {
            input1_size: u32,
            input2_size: u32,
            axis_dim1: u32,
            axis_dim2: u32,
            stride: u32,
            _pad: [u32; 3], // Padding to 32 bytes
        }

        let params = ConcatParams {
            input1_size: size1 as u32,
            input2_size: size2 as u32,
            axis_dim1: 0, // Not used for simple 1D concat
            axis_dim2: 0,
            stride: 0,
            _pad: [0; 3],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Concat Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Concat Layout"),
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
            label: Some("Concat Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input2_buffer.as_entire_binding(),
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

        // Create pipeline with concat_1d entry point
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Concat Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Concat Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Concat Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "concat_1d",
                compilation_options: Default::default(),
                cache: None,
            });

        let workgroups = self.calculate_workgroups(output_size, 256);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Concat Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Concat Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (output_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, output_size).await
    }

    /// Execute Slice: Extract a slice from a tensor
    ///
    /// Extracts a contiguous section of a tensor [start..end).
    /// Essential for tensor manipulation, attention windows, sequence processing.
    ///
    /// Deep Debt: Slice bounds determined at runtime, flexible extraction.
    ///
    /// Use cases: Attention windows, sequence chunking, tensor manipulation.
    pub async fn execute_slice(&self, input: &[f32], start: usize, end: usize) -> Result<Vec<f32>> {
        anyhow::ensure!(!input.is_empty(), "Slice: input cannot be empty");
        anyhow::ensure!(start < input.len(), "Slice: start index out of bounds");
        anyhow::ensure!(end <= input.len(), "Slice: end index out of bounds");
        anyhow::ensure!(start < end, "Slice: start must be less than end");

        let output_size = end - start;
        let shader_source = include_str!("../shaders/slice.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Slice Input");
        let output_buffer = self.create_output_buffer(output_size, "Slice Output");
        let staging_buffer = self.create_staging_buffer(output_size, "Slice Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SliceParams {
            start: u32,
            end: u32,
            stride: u32,
            _pad: u32, // Padding to 16 bytes
        }

        let params = SliceParams {
            start: start as u32,
            end: end as u32,
            stride: 1, // For simple 1D slice
            _pad: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slice Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Slice Layout"),
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
            label: Some("Slice Bind Group"),
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

        // Create pipeline with slice_1d entry point
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slice Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Slice Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Slice Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "slice_1d",
                compilation_options: Default::default(),
                cache: None,
            });

        let workgroups = self.calculate_workgroups(output_size, 256);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Slice Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slice Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (output_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, output_size).await
    }

    /// Execute Pad: Add padding to a tensor
    ///
    /// Pads a tensor with constant values (typically zeros).
    /// Essential for maintaining spatial dimensions in convolutions.
    ///
    /// Deep Debt: Padding amounts determined at runtime.
    ///
    /// Use cases: "Same" padding in CNNs, zero-padding for convolutions.
    pub async fn execute_pad(
        &self,
        input: &[f32],
        input_height: usize,
        input_width: usize,
        pad_top: usize,
        pad_bottom: usize,
        pad_left: usize,
        pad_right: usize,
        pad_value: f32,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(!input.is_empty(), "Pad: input cannot be empty");
        anyhow::ensure!(
            input.len() == input_height * input_width,
            "Pad: input size must match height * width"
        );

        let output_height = input_height + pad_top + pad_bottom;
        let output_width = input_width + pad_left + pad_right;
        let output_size = output_height * output_width;

        let shader_source = include_str!("../shaders/pad.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Pad Input");
        let output_buffer = self.create_output_buffer(output_size, "Pad Output");
        let staging_buffer = self.create_staging_buffer(output_size, "Pad Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct PadParams {
            input_height: u32,  // offset 0
            input_width: u32,   // offset 4
            output_height: u32, // offset 8
            output_width: u32,  // offset 12
            pad_top: u32,       // offset 16
            pad_left: u32,      // offset 20
            pad_value: f32,     // offset 24
            _pad: [u32; 3],     // offset 28, padding to 40 bytes
        }

        let params = PadParams {
            input_height: input_height as u32,
            input_width: input_width as u32,
            output_height: output_height as u32,
            output_width: output_width as u32,
            pad_top: pad_top as u32,
            pad_left: pad_left as u32,
            pad_value,
            _pad: [0; 3],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pad Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Pad Layout"),
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
            label: Some("Pad Bind Group"),
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

        // Create pipeline with pad_2d entry point
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pad Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pad Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pad Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "pad_2d",
                compilation_options: Default::default(),
                cache: None,
            });

        // Dispatch with 2D workgroups
        let workgroup_x = output_width.div_ceil(8);
        let workgroup_y = output_height.div_ceil(8);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pad Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pad Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroup_x as u32, workgroup_y as u32, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (output_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, output_size).await
    }

    /// Execute Reshape: Reshape a tensor to new dimensions
    ///
    /// Changes tensor shape without copying data (memory layout preserved).
    /// Essential for model flexibility and dimension manipulation.
    ///
    /// Deep Debt: Shapes determined at runtime, zero hardcoding.
    ///
    /// Use cases: Model output reshaping, dimension manipulation, view changes.
    pub async fn execute_reshape(&self, input: &[f32], new_shape: &[usize]) -> Result<Vec<f32>> {
        anyhow::ensure!(!input.is_empty(), "Reshape: input cannot be empty");
        anyhow::ensure!(!new_shape.is_empty(), "Reshape: new_shape cannot be empty");

        let input_size = input.len();
        let output_size: usize = new_shape.iter().product();

        anyhow::ensure!(
            input_size == output_size,
            "Reshape: input size ({}) must match output size ({})",
            input_size,
            output_size
        );

        let shader_source = include_str!("../shaders/reshape.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Reshape Input");
        let output_buffer = self.create_output_buffer(output_size, "Reshape Output");
        let staging_buffer = self.create_staging_buffer(output_size, "Reshape Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ReshapeParams {
            size: u32,
            _pad: [u32; 3],
        }

        let params = ReshapeParams {
            size: input_size as u32,
            _pad: [0; 3],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Reshape Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Reshape Layout"),
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
            label: Some("Reshape Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Reshape", &bind_group_layout);
        let workgroups = self.calculate_workgroups(output_size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Reshape");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (output_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, output_size).await
    }

    /// Execute Split: Split tensor into two parts (inverse of Concat)
    ///
    /// Splits input at specified point into two output tensors.
    /// Essential for multi-path networks and dynamic routing.
    ///
    /// Deep Debt: Split point determined at runtime.
    ///
    /// Use cases: Multi-path networks, separate feature groups, dynamic routing.
    pub async fn execute_split(
        &self,
        input: &[f32],
        split_point: usize,
    ) -> Result<(Vec<f32>, Vec<f32>)> {
        anyhow::ensure!(!input.is_empty(), "Split: input cannot be empty");
        anyhow::ensure!(
            split_point > 0 && split_point < input.len(),
            "Split: invalid split point"
        );

        let total_size = input.len();
        let size1 = split_point;
        let size2 = total_size - split_point;

        let shader_source = include_str!("../shaders/split.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Split Input");
        let output1_buffer = self.create_output_buffer(size1, "Split Output1");
        let output2_buffer = self.create_output_buffer(size2, "Split Output2");
        let staging1_buffer = self.create_staging_buffer(size1, "Split Staging1");
        let staging2_buffer = self.create_staging_buffer(size2, "Split Staging2");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SplitParams {
            total_size: u32,
            split_point: u32,
            _pad: u32,
            _pad2: u32,
        }

        let params = SplitParams {
            total_size: total_size as u32,
            split_point: split_point as u32,
            _pad: 0,
            _pad2: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Split Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Split Layout"),
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
            label: Some("Split Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output2_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Split", &bind_group_layout);
        let workgroups = self.calculate_workgroups(total_size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Split");

        encoder.copy_buffer_to_buffer(
            &output1_buffer,
            0,
            &staging1_buffer,
            0,
            (size1 * std::mem::size_of::<f32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &output2_buffer,
            0,
            &staging2_buffer,
            0,
            (size2 * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));

        let output1 = self.read_buffer(&staging1_buffer, size1).await?;
        let output2 = self.read_buffer(&staging2_buffer, size2).await?;

        Ok((output1, output2))
    }

    /// Execute Squeeze: Remove singleton dimensions
    ///
    /// Removes dimensions of size 1 from tensor shape (data unchanged).
    /// Essential for dimension cleanup and shape normalization.
    ///
    /// Deep Debt: Shape manipulation at runtime.
    ///
    /// Use cases: Remove broadcast dimensions, shape normalization, dimension cleanup.
    pub async fn execute_squeeze(&self, input: &[f32]) -> Result<Vec<f32>> {
        // Squeeze is a metadata operation - data unchanged, just shape
        // Return a copy of the input data
        Ok(input.to_vec())
    }

    /// Execute Unsqueeze: Add singleton dimensions
    ///
    /// Adds dimensions of size 1 to tensor shape (data unchanged).
    /// Essential for broadcasting and dimension expansion.
    ///
    /// Deep Debt: Shape manipulation at runtime.
    ///
    /// Use cases: Broadcasting preparation, dimension expansion, tensor alignment.
    pub async fn execute_unsqueeze(&self, input: &[f32]) -> Result<Vec<f32>> {
        // Unsqueeze is a metadata operation - data unchanged, just shape
        // Return a copy of the input data
        Ok(input.to_vec())
    }
}
