//! Basic tensor operations
//!
//! MatMul, Vector Addition, Binary Operations, etc.
//! Core building blocks for neural networks.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::BinaryOp};

impl WgpuExecutor {
    /// Execute matrix multiplication: C = A * B
    ///
    /// Modern idiomatic Rust with safe buffer handling.
    /// Deep Debt: Matrix dimensions determined at runtime, not hardcoded.
    pub async fn execute_matmul(
        &self,
        a: &[f32],
        b: &[f32],
        m: usize, // A is m x k
        n: usize, // B is k x n
        k: usize,
    ) -> Result<Vec<f32>> {
        let shader_source = include_str!("../shaders/matmul.wgsl");

        // Create buffers
        let a_buffer = self.create_input_buffer(a, "MatMul A");
        let b_buffer = self.create_input_buffer(b, "MatMul B");
        let c_buffer = self.create_output_buffer(m * n, "MatMul C");
        let staging_buffer = self.create_staging_buffer(m * n, "MatMul Staging");

        // Create params buffer (dimensions - runtime configuration, not hardcoded!)
        let params = [m as u32, n as u32, k as u32, 0]; // Pad to 16 bytes
        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MatMul Params"),
                    contents: bytemuck::cast_slice(&params),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MatMul Bind Group Layout"),
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
            label: Some("MatMul Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "MatMul", &bind_group_layout);

        // 2D workgroups for matrix multiply (runtime calculated)
        let tile_size = 16u32;
        let workgroups_x = (n as u32 + tile_size - 1) / tile_size;
        let workgroups_y = (m as u32 + tile_size - 1) / tile_size;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MatMul Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MatMul Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        encoder.copy_buffer_to_buffer(
            &c_buffer,
            0,
            &staging_buffer,
            0,
            (m * n * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, m * n).await
    }

    /// Execute vector addition: C = A * alpha + B
    ///
    /// Efficient SAXPY operation (scalar alpha x plus y).
    /// Deep Debt: Alpha determined at runtime, not compile-time constant.
    pub async fn execute_add(&self, a: &[f32], b: &[f32], alpha: f32) -> Result<Vec<f32>> {
        assert_eq!(a.len(), b.len(), "Vectors must be same length");
        let size = a.len();

        let shader_source = include_str!("../shaders/add.wgsl");

        let a_buffer = self.create_input_buffer(a, "Add A");
        let b_buffer = self.create_input_buffer(b, "Add B");
        let c_buffer = self.create_output_buffer(size, "Add C");
        let staging_buffer = self.create_staging_buffer(size, "Add Staging");

        // Runtime parameter (not hardcoded!)
        let params = [alpha, 0.0, 0.0, 0.0]; // Pad to 16 bytes
        let params_buffer =
            self.device
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
            (size * std::mem::size_of::<f32>()) as u64,
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
        anyhow::ensure!(a.len() == b.len(), "Vector sizes must match for binary operation");
        let size = a.len();

        let shader_source = include_str!("../shaders/elementwise_binary.wgsl");

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

        let params_buffer =
            self.device
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
            (size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, size).await
    }

    /// Execute transpose: (rows, cols) -> (cols, rows)
    ///
    /// Deep Debt: Dimensions determined at runtime.
    pub async fn execute_transpose(
        &self,
        input: &[f32],
        rows: usize,
        cols: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == rows * cols,
            "Input size must match rows * cols"
        );

        let shader_source = include_str!("../shaders/transpose.wgsl");

        let input_buffer = self.create_input_buffer(input, "Transpose Input");
        let output_buffer = self.create_output_buffer(rows * cols, "Transpose Output");
        let staging_buffer = self.create_staging_buffer(rows * cols, "Transpose Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TransposeParams {
            rows: u32,
            cols: u32,
        }

        let params = TransposeParams {
            rows: rows as u32,
            cols: cols as u32,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Transpose Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Transpose Layout"),
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
            label: Some("Transpose Bind Group"),
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

        let pipeline =
            self.create_simple_pipeline(shader_source, "Transpose", &bind_group_layout);

        // 2D workgroups for better memory access patterns
        let tile_size = 16u32;
        let workgroups_x = (cols as u32 + tile_size - 1) / tile_size;
        let workgroups_y = (rows as u32 + tile_size - 1) / tile_size;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Transpose Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Transpose Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (rows * cols * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, rows * cols).await
    }

    /// Execute Conv1D: 1D convolution for sequences
    ///
    /// Convolution operation for time-series, NLP, and audio processing.
    /// Input shape: [batch, in_channels, length]
    /// Weight shape: [out_channels, in_channels, kernel_size]
    /// Output shape: [batch, out_channels, out_length]
    ///
    /// Deep Debt: All dimensions and hyperparameters determined at runtime.
    pub async fn execute_conv1d(
        &self,
        input: &[f32],
        weight: &[f32],
        bias: &[f32],
        batch: usize,
        in_channels: usize,
        out_channels: usize,
        in_length: usize,
        config: super::types::Conv1DConfig,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * in_channels * in_length,
            "Conv1D: input size must match batch * in_channels * in_length"
        );
        anyhow::ensure!(
            weight.len() == out_channels * in_channels * config.kernel_size,
            "Conv1D: weight size must match out_channels * in_channels * kernel_size"
        );
        anyhow::ensure!(
            bias.len() == out_channels,
            "Conv1D: bias size must match out_channels"
        );

        // Calculate output length
        let out_length = (in_length + 2 * config.padding - config.dilation * (config.kernel_size - 1) - 1) / config.stride + 1;
        let out_size = batch * out_channels * out_length;

        let shader_source = include_str!("../shaders/conv1d.wgsl");

        let input_buffer = self.create_input_buffer(input, "Conv1D Input");
        let weight_buffer = self.create_input_buffer(weight, "Conv1D Weight");
        let bias_buffer = self.create_input_buffer(bias, "Conv1D Bias");
        let output_buffer = self.create_output_buffer(out_size, "Conv1D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "Conv1D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Conv1DParams {
            batch: u32,
            in_channels: u32,
            out_channels: u32,
            in_length: u32,
            kernel_size: u32,
            stride: u32,
            padding: u32,
            dilation: u32,
            out_length: u32,
            _padding: [u32; 3],
        }

        let params = Conv1DParams {
            batch: batch as u32,
            in_channels: in_channels as u32,
            out_channels: out_channels as u32,
            in_length: in_length as u32,
            kernel_size: config.kernel_size as u32,
            stride: config.stride as u32,
            padding: config.padding as u32,
            dilation: config.dilation as u32,
            out_length: out_length as u32,
            _padding: [0; 3],
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Conv1D Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Conv1D Layout"),
                    entries: &[
                        // Input
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
                        // Weight
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
                        // Bias
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
                        // Output
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
                        // Params
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
            label: Some("Conv1D Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bias_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = self.create_simple_pipeline(shader_source, "Conv1D", &bind_group_layout);
        let workgroups = self.calculate_workgroups(out_size, 256);
        let mut encoder = self.execute_compute_pass(&pipeline, &bind_group, workgroups, "Conv1D");

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (out_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, out_size).await
    }

    /// Execute Depthwise Conv2D: Efficient channel-wise convolution
    ///
    /// Applies a separate filter to each input channel (no channel mixing).
    /// Input shape: [batch, channels, height, width]
    /// Weight shape: [channels, 1, kernel_h, kernel_w]
    /// Output shape: [batch, channels, out_height, out_width]
    ///
    /// Used in: MobileNet, EfficientNet, lightweight CNNs.
    /// Benefits: Dramatically reduces parameters vs standard Conv2D.
    ///
    /// Deep Debt: All dimensions and hyperparameters determined at runtime.
    pub async fn execute_depthwise_conv2d(
        &self,
        input: &[f32],
        weight: &[f32],
        bias: &[f32],
        batch: usize,
        channels: usize,
        in_height: usize,
        in_width: usize,
        config: super::types::DepthwiseConv2DConfig,
    ) -> Result<Vec<f32>> {
        let (kernel_h, kernel_w) = config.kernel_size;
        let (stride_h, stride_w) = config.stride;
        let (pad_h, pad_w) = config.padding;

        anyhow::ensure!(
            input.len() == batch * channels * in_height * in_width,
            "DepthwiseConv2D: input size mismatch"
        );
        anyhow::ensure!(
            weight.len() == channels * kernel_h * kernel_w,
            "DepthwiseConv2D: weight size must be channels * kernel_h * kernel_w"
        );
        anyhow::ensure!(
            bias.len() == channels,
            "DepthwiseConv2D: bias size must match channels"
        );

        // Calculate output dimensions
        let out_height = (in_height + 2 * pad_h - kernel_h) / stride_h + 1;
        let out_width = (in_width + 2 * pad_w - kernel_w) / stride_w + 1;
        let out_size = batch * channels * out_height * out_width;

        let shader_source = include_str!("../shaders/depthwise_conv2d.wgsl");

        let input_buffer = self.create_input_buffer(input, "DepthwiseConv2D Input");
        let weight_buffer = self.create_input_buffer(weight, "DepthwiseConv2D Weight");
        let bias_buffer = self.create_input_buffer(bias, "DepthwiseConv2D Bias");
        let output_buffer = self.create_output_buffer(out_size, "DepthwiseConv2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "DepthwiseConv2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DepthwiseConv2DParams {
            batch: u32,
            channels: u32,
            in_height: u32,
            in_width: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_h: u32,
            stride_w: u32,
            pad_h: u32,
            pad_w: u32,
            out_height: u32,
            out_width: u32,
        }

        let params = DepthwiseConv2DParams {
            batch: batch as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            kernel_h: kernel_h as u32,
            kernel_w: kernel_w as u32,
            stride_h: stride_h as u32,
            stride_w: stride_w as u32,
            pad_h: pad_h as u32,
            pad_w: pad_w as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
        };

        let params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("DepthwiseConv2D Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("DepthwiseConv2D Layout"),
                    entries: &[
                        // Input
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
                        // Weight
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
                        // Bias
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
                        // Output
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
                        // Params
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
            label: Some("DepthwiseConv2D Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bias_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            self.create_simple_pipeline(shader_source, "DepthwiseConv2D", &bind_group_layout);

        // 2D workgroups for spatial operations
        let workgroups_x = (out_width as u32 + 15) / 16;
        let workgroups_y = (out_height as u32 + 15) / 16;
        let workgroups_z = (batch * channels) as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("DepthwiseConv2D Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("DepthwiseConv2D Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (out_size * std::mem::size_of::<f32>()) as u64,
        );

        self.queue.submit(Some(encoder.finish()));
        self.read_buffer(&staging_buffer, out_size).await
    }
}
