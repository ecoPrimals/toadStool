// SPDX-License-Identifier: AGPL-3.0-or-later
//! 2D Convolutions: depthwise, standard, transposed

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::executor::WgpuExecutor;

impl WgpuExecutor {
    pub async fn execute_depthwise_conv2d(
        &self,
        input: &[f32],
        weight: &[f32],
        bias: &[f32],
        batch: usize,
        channels: usize,
        in_height: usize,
        in_width: usize,
        config: super::super::types::DepthwiseConv2DConfig,
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

        let shader_source = include_str!("../../shaders/depthwise_conv2d.wgsl");

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

        let params_buffer = self
            .device
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
        let workgroups_x = (out_width as u32).div_ceil(16);
        let workgroups_y = (out_height as u32).div_ceil(16);
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

    /// Execute Conv2D: Standard 2D convolution
    ///
    /// The fundamental building block for CNNs (ResNet, VGG, etc.).
    /// Applies learned filters across spatial dimensions to extract features.
    ///
    /// Deep Debt: All dimensions (filters, stride, padding) determined at runtime.
    /// No hardcoding, fully configurable per-invocation.
    ///
    /// Use cases: Feature extraction in ResNet, VGG, YOLO, etc.
    pub async fn execute_conv2d(
        &self,
        input: &[f32],
        weights: &[f32],
        bias: &[f32],
        batch: usize,
        in_channels: usize,
        out_channels: usize,
        input_height: usize,
        input_width: usize,
        config: super::super::types::Conv2DConfig,
    ) -> Result<Vec<f32>> {
        let (kernel_h, kernel_w) = config.kernel_size;
        let (stride_h, stride_w) = config.stride;
        let (pad_h, pad_w) = config.padding;
        let (dilation_h, dilation_w) = config.dilation;

        // Calculate output dimensions (Deep Debt: computed at runtime!)
        let out_height =
            (input_height + 2 * pad_h - dilation_h * (kernel_h - 1) - 1) / stride_h + 1;
        let out_width = (input_width + 2 * pad_w - dilation_w * (kernel_w - 1) - 1) / stride_w + 1;

        anyhow::ensure!(
            input.len() == batch * in_channels * input_height * input_width,
            "Conv2D: input size mismatch"
        );
        anyhow::ensure!(
            weights.len() == out_channels * in_channels * kernel_h * kernel_w,
            "Conv2D: weight size must be out_channels * in_channels * kernel_h * kernel_w"
        );
        anyhow::ensure!(
            bias.len() == out_channels,
            "Conv2D: bias size must match out_channels"
        );

        let out_size = batch * out_channels * out_height * out_width;
        let shader_source = include_str!("../../shaders/conv2d.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Conv2D Input");
        let weight_buffer = self.create_input_buffer(weights, "Conv2D Weight");
        let bias_buffer = self.create_input_buffer(bias, "Conv2D Bias");
        let output_buffer = self.create_output_buffer(out_size, "Conv2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "Conv2D Staging");

        // Parameters struct matching WGSL
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Conv2DParams {
            batch_size: u32,
            in_channels: u32,
            out_channels: u32,
            input_h: u32,
            input_w: u32,
            output_h: u32,
            output_w: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_h: u32,
            stride_w: u32,
            padding_h: u32,
            padding_w: u32,
            dilation_h: u32,
            dilation_w: u32,
            _pad: u32, // Padding to 64 bytes (16 * 4)
        }

        let params = Conv2DParams {
            batch_size: batch as u32,
            in_channels: in_channels as u32,
            out_channels: out_channels as u32,
            input_h: input_height as u32,
            input_w: input_width as u32,
            output_h: out_height as u32,
            output_w: out_width as u32,
            kernel_h: kernel_h as u32,
            kernel_w: kernel_w as u32,
            stride_h: stride_h as u32,
            stride_w: stride_w as u32,
            padding_h: pad_h as u32,
            padding_w: pad_w as u32,
            dilation_h: dilation_h as u32,
            dilation_w: dilation_w as u32,
            _pad: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Conv2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Conv2D Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Conv2D Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Conv2D", &bind_group_layout);

        // Dispatch with 2D workgroups for spatial dimensions
        // Each workgroup handles 16x16 output pixels
        let workgroup_x = out_width.div_ceil(16);
        let workgroup_y = out_height.div_ceil(16);
        let workgroup_z = out_channels;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Conv2D Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Conv2D Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroup_x as u32, workgroup_y as u32, workgroup_z as u32);
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

    /// Execute TransposedConv2D: Transposed 2D Convolution (Deconvolution/Upsampling)
    ///
    /// Performs learnable upsampling via transposed convolution.
    /// Essential for U-Net decoder, image super-resolution, GANs.
    ///
    /// Deep Debt: All dimensions runtime-configured, zero hardcoding.
    ///
    /// Use cases: U-Net upsampling, semantic segmentation decoders, GANs.
    pub async fn execute_transposed_conv2d(
        &self,
        input: &[f32],
        weights: &[f32],
        bias: &[f32],
        batch: usize,
        in_channels: usize,
        out_channels: usize,
        input_height: usize,
        input_width: usize,
        config: super::super::types::TransposedConv2DConfig,
    ) -> Result<Vec<f32>> {
        // Calculate output dimensions for transposed convolution
        // output_size = (input_size - 1) * stride - 2 * padding + kernel_size + output_padding
        let output_height = (input_height - 1) * config.stride.0 - 2 * config.padding.0
            + config.kernel_size.0
            + config.output_padding.0;
        let output_width = (input_width - 1) * config.stride.1 - 2 * config.padding.1
            + config.kernel_size.1
            + config.output_padding.1;

        let input_size = batch * in_channels * input_height * input_width;
        let weight_size = in_channels * out_channels * config.kernel_size.0 * config.kernel_size.1;
        let out_size = batch * out_channels * output_height * output_width;

        anyhow::ensure!(
            input.len() == input_size,
            "TransposedConv2D: input size mismatch"
        );
        anyhow::ensure!(
            weights.len() == weight_size,
            "TransposedConv2D: weights size mismatch"
        );
        anyhow::ensure!(
            bias.len() == out_channels,
            "TransposedConv2D: bias size mismatch"
        );

        let shader_source = include_str!("../../shaders/transposed_conv2d.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "TransposedConv2D Input");
        let weight_buffer = self.create_input_buffer(weights, "TransposedConv2D Weights");
        let bias_buffer = self.create_input_buffer(bias, "TransposedConv2D Bias");
        let output_buffer = self.create_output_buffer(out_size, "TransposedConv2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "TransposedConv2D Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TransposedConv2DParams {
            batch_size: u32,
            in_channels: u32,
            out_channels: u32,
            input_h: u32,
            input_w: u32,
            output_h: u32,
            output_w: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_h: u32,
            stride_w: u32,
            padding_h: u32,
            padding_w: u32,
            output_padding_h: u32,
            output_padding_w: u32,
            _pad: u32,
        }

        let params = TransposedConv2DParams {
            batch_size: batch as u32,
            in_channels: in_channels as u32,
            out_channels: out_channels as u32,
            input_h: input_height as u32,
            input_w: input_width as u32,
            output_h: output_height as u32,
            output_w: output_width as u32,
            kernel_h: config.kernel_size.0 as u32,
            kernel_w: config.kernel_size.1 as u32,
            stride_h: config.stride.0 as u32,
            stride_w: config.stride.1 as u32,
            padding_h: config.padding.0 as u32,
            padding_w: config.padding.1 as u32,
            output_padding_h: config.output_padding.0 as u32,
            output_padding_w: config.output_padding.1 as u32,
            _pad: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("TransposedConv2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TransposedConv2D Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TransposedConv2D Bind Group"),
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
            self.create_simple_pipeline(shader_source, "TransposedConv2D", &bind_group_layout);

        // Dispatch with 2D workgroups + output channels
        let workgroup_x = output_width.div_ceil(16);
        let workgroup_y = output_height.div_ceil(16);
        let workgroup_z = out_channels;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("TransposedConv2D Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("TransposedConv2D Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroup_x as u32, workgroup_y as u32, workgroup_z as u32);
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
