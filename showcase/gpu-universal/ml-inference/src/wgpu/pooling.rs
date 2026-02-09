//! Pooling operations
//!
//! MaxPool2D and other pooling operations for CNNs.
//! Downsampling with spatial reduction.

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::{executor::WgpuExecutor, types::Pool2DConfig};

impl WgpuExecutor {
    /// Execute MaxPool2D: 2D max pooling operation
    ///
    /// Downsamples spatial dimensions by taking maximum value in each window.
    /// Common in CNNs for translation invariance and parameter reduction.
    ///
    /// Deep Debt: All dimensions (kernel, stride, padding) determined at runtime.
    pub async fn execute_max_pool_2d(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        height: usize,
        width: usize,
        config: Pool2DConfig,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * height * width,
            "MaxPool2D: input size must match batch * channels * height * width"
        );

        let (kernel_h, kernel_w) = config.kernel_size;
        let (stride_h, stride_w) = config.stride;
        let (pad_h, pad_w) = config.padding;

        // Calculate output dimensions (Deep Debt: computed at runtime!)
        let out_height = (height + 2 * pad_h - kernel_h) / stride_h + 1;
        let out_width = (width + 2 * pad_w - kernel_w) / stride_w + 1;
        let out_size = batch * channels * out_height * out_width;

        let shader_source = include_str!("../shaders/maxpool2d.wgsl");

        let input_buffer = self.create_input_buffer(input, "MaxPool2D Input");
        let output_buffer = self.create_output_buffer(out_size, "MaxPool2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "MaxPool2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct MaxPool2DParams {
            batch_size: u32,
            channels: u32,
            input_height: u32,
            input_width: u32,
            output_height: u32,
            output_width: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_h: u32,
            stride_w: u32,
            padding_h: u32,
            padding_w: u32,
        }

        let params = MaxPool2DParams {
            batch_size: batch as u32,
            channels: channels as u32,
            input_height: height as u32,
            input_width: width as u32,
            output_height: out_height as u32,
            output_width: out_width as u32,
            kernel_h: kernel_h as u32,
            kernel_w: kernel_w as u32,
            stride_h: stride_h as u32,
            stride_w: stride_w as u32,
            padding_h: pad_h as u32,
            padding_w: pad_w as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MaxPool2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("MaxPool2D Layout"),
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
            label: Some("MaxPool2D Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "MaxPool2D", &bind_group_layout);

        // 2D workgroups for spatial operations
        let workgroups_x = (out_width as u32).div_ceil(16);
        let workgroups_y = (out_height as u32).div_ceil(16);
        let workgroups_z = (batch * channels) as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MaxPool2D Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MaxPool2D Pass"),
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

    /// Execute Global Average Pooling
    ///
    /// Reduces spatial dimensions (H x W) to 1x1 by averaging across all locations.
    /// Output shape: [batch, channels, 1, 1]
    ///
    /// Used in: Modern CNNs (ResNet, EfficientNet) to replace FC layers.
    /// Benefits: Reduces parameters, increases spatial invariance.
    ///
    /// Deep Debt: All dimensions determined at runtime, vendor-agnostic GPU execution.
    pub async fn execute_global_avg_pool(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        height: usize,
        width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * height * width,
            "GlobalAvgPool: input size must match batch * channels * height * width"
        );

        let out_size = batch * channels;
        let shader_source = include_str!("../shaders/global_avgpool.wgsl");

        let input_buffer = self.create_input_buffer(input, "GlobalAvgPool Input");
        let output_buffer = self.create_output_buffer(out_size, "GlobalAvgPool Output");
        let staging_buffer = self.create_staging_buffer(out_size, "GlobalAvgPool Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GlobalPoolParams {
            batch_size: u32,
            channels: u32,
            height: u32,
            width: u32,
        }

        let params = GlobalPoolParams {
            batch_size: batch as u32,
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GlobalAvgPool Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GlobalAvgPool Layout"),
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
            label: Some("GlobalAvgPool Bind Group"),
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
            self.create_simple_pipeline(shader_source, "GlobalAvgPool", &bind_group_layout);

        let workgroups = self.calculate_workgroups(out_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "GlobalAvgPool");

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

    /// Execute Global Max Pooling
    ///
    /// Reduces spatial dimensions (H x W) to 1x1 by taking maximum across all locations.
    /// Output shape: [batch, channels, 1, 1]
    ///
    /// Used in: CNNs for classification, attention mechanisms.
    /// Benefits: Captures most salient features, reduces overfitting.
    ///
    /// Deep Debt: All dimensions determined at runtime, vendor-agnostic GPU execution.
    pub async fn execute_global_max_pool(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        height: usize,
        width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * height * width,
            "GlobalMaxPool: input size must match batch * channels * height * width"
        );

        let out_size = batch * channels;
        let shader_source = include_str!("../shaders/global_maxpool.wgsl");

        let input_buffer = self.create_input_buffer(input, "GlobalMaxPool Input");
        let output_buffer = self.create_output_buffer(out_size, "GlobalMaxPool Output");
        let staging_buffer = self.create_staging_buffer(out_size, "GlobalMaxPool Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct GlobalPoolParams {
            batch_size: u32,
            channels: u32,
            height: u32,
            width: u32,
        }

        let params = GlobalPoolParams {
            batch_size: batch as u32,
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GlobalMaxPool Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GlobalMaxPool Layout"),
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
            label: Some("GlobalMaxPool Bind Group"),
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
            self.create_simple_pipeline(shader_source, "GlobalMaxPool", &bind_group_layout);

        let workgroups = self.calculate_workgroups(out_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "GlobalMaxPool");

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

    /// Execute Adaptive Average Pooling 2D
    ///
    /// Pools input to a specific output size regardless of input dimensions.
    /// Automatically computes kernel and stride to produce desired output size.
    ///
    /// Used in: Classification networks, SPPNet, PSPNet, variable input sizes.
    /// Benefits: Network can handle any input size, produces fixed output.
    ///
    /// Deep Debt: All dimensions determined at runtime, fully adaptive.
    pub async fn execute_adaptive_avg_pool_2d(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        in_height: usize,
        in_width: usize,
        out_height: usize,
        out_width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * in_height * in_width,
            "AdaptiveAvgPool2D: input size mismatch"
        );

        let out_size = batch * channels * out_height * out_width;
        let shader_source = include_str!("../shaders/adaptive_avgpool2d.wgsl");

        let input_buffer = self.create_input_buffer(input, "AdaptiveAvgPool2D Input");
        let output_buffer = self.create_output_buffer(out_size, "AdaptiveAvgPool2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "AdaptiveAvgPool2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AdaptivePoolParams {
            batch: u32,
            channels: u32,
            in_height: u32,
            in_width: u32,
            out_height: u32,
            out_width: u32,
        }

        let params = AdaptivePoolParams {
            batch: batch as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("AdaptiveAvgPool2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("AdaptiveAvgPool2D Layout"),
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
            label: Some("AdaptiveAvgPool2D Bind Group"),
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
            self.create_simple_pipeline(shader_source, "AdaptiveAvgPool2D", &bind_group_layout);

        let workgroups_x = (out_width as u32).div_ceil(16);
        let workgroups_y = (out_height as u32).div_ceil(16);
        let workgroups_z = (batch * channels) as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("AdaptiveAvgPool2D Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AdaptiveAvgPool2D Pass"),
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

    /// Execute Adaptive Max Pooling 2D
    ///
    /// Pools input to a specific output size using maximum operation.
    /// Automatically computes pooling regions to produce desired output size.
    ///
    /// Used in: Classification, SPPNet, flexible architectures.
    /// Benefits: Variable input sizes, captures most salient features.
    ///
    /// Deep Debt: All dimensions determined at runtime, fully adaptive.
    pub async fn execute_adaptive_max_pool_2d(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        in_height: usize,
        in_width: usize,
        out_height: usize,
        out_width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * in_height * in_width,
            "AdaptiveMaxPool2D: input size mismatch"
        );

        let out_size = batch * channels * out_height * out_width;
        let shader_source = include_str!("../shaders/adaptive_maxpool2d.wgsl");

        let input_buffer = self.create_input_buffer(input, "AdaptiveMaxPool2D Input");
        let output_buffer = self.create_output_buffer(out_size, "AdaptiveMaxPool2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "AdaptiveMaxPool2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AdaptivePoolParams {
            batch: u32,
            channels: u32,
            in_height: u32,
            in_width: u32,
            out_height: u32,
            out_width: u32,
        }

        let params = AdaptivePoolParams {
            batch: batch as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("AdaptiveMaxPool2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("AdaptiveMaxPool2D Layout"),
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
            label: Some("AdaptiveMaxPool2D Bind Group"),
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
            self.create_simple_pipeline(shader_source, "AdaptiveMaxPool2D", &bind_group_layout);

        let workgroups_x = (out_width as u32).div_ceil(16);
        let workgroups_y = (out_height as u32).div_ceil(16);
        let workgroups_z = (batch * channels) as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("AdaptiveMaxPool2D Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AdaptiveMaxPool2D Pass"),
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

    /// Execute AvgPool2D: 2D average pooling operation
    ///
    /// Downsamples spatial dimensions by computing average value in each window.
    /// Complementary to MaxPool2D - used when averaging is preferred over max.
    ///
    /// Deep Debt: All dimensions (kernel, stride, padding) determined at runtime.
    pub async fn execute_avg_pool_2d(
        &self,
        input: &[f32],
        batch: usize,
        channels: usize,
        height: usize,
        width: usize,
        config: Pool2DConfig,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch * channels * height * width,
            "AvgPool2D: input size must match batch * channels * height * width"
        );

        let (kernel_h, kernel_w) = config.kernel_size;
        let (stride_h, stride_w) = config.stride;
        let (pad_h, pad_w) = config.padding;

        // Calculate output dimensions
        let out_height = (height + 2 * pad_h - kernel_h) / stride_h + 1;
        let out_width = (width + 2 * pad_w - kernel_w) / stride_w + 1;
        let out_size = batch * channels * out_height * out_width;

        let shader_source = include_str!("../shaders/avgpool2d.wgsl");

        let input_buffer = self.create_input_buffer(input, "AvgPool2D Input");
        let output_buffer = self.create_output_buffer(out_size, "AvgPool2D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "AvgPool2D Staging");

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct AvgPool2DParams {
            batch_size: u32,
            channels: u32,
            input_height: u32,
            input_width: u32,
            output_height: u32,
            output_width: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_h: u32,
            stride_w: u32,
            padding_h: u32,
            padding_w: u32,
        }

        let params = AvgPool2DParams {
            batch_size: batch as u32,
            channels: channels as u32,
            input_height: height as u32,
            input_width: width as u32,
            output_height: out_height as u32,
            output_width: out_width as u32,
            kernel_h: kernel_h as u32,
            kernel_w: kernel_w as u32,
            stride_h: stride_h as u32,
            stride_w: stride_w as u32,
            padding_h: pad_h as u32,
            padding_w: pad_w as u32,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("AvgPool2D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("AvgPool2D Layout"),
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
            label: Some("AvgPool2D Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "AvgPool2D", &bind_group_layout);
        let workgroups = self.calculate_workgroups(out_size, 256);
        let mut encoder =
            self.execute_compute_pass(&pipeline, &bind_group, workgroups, "AvgPool2D");

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
