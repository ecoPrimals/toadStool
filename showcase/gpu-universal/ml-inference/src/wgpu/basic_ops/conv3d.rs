//! 3D Convolution

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::executor::WgpuExecutor;

impl WgpuExecutor {
    pub async fn execute_conv3d(
        &self,
        input: &[f32],
        weights: &[f32],
        bias: &[f32],
        batch: usize,
        in_channels: usize,
        out_channels: usize,
        input_depth: usize,
        input_height: usize,
        input_width: usize,
        config: super::super::types::Conv3DConfig,
    ) -> Result<Vec<f32>> {
        // Calculate output dimensions
        let output_depth = (input_depth + 2 * config.padding.0
            - config.dilation.0 * (config.kernel_size.0 - 1)
            - 1)
            / config.stride.0
            + 1;
        let output_height = (input_height + 2 * config.padding.1
            - config.dilation.1 * (config.kernel_size.1 - 1)
            - 1)
            / config.stride.1
            + 1;
        let output_width = (input_width + 2 * config.padding.2
            - config.dilation.2 * (config.kernel_size.2 - 1)
            - 1)
            / config.stride.2
            + 1;

        let input_size = batch * in_channels * input_depth * input_height * input_width;
        let weight_size = out_channels
            * in_channels
            * config.kernel_size.0
            * config.kernel_size.1
            * config.kernel_size.2;
        let out_size = batch * out_channels * output_depth * output_height * output_width;

        anyhow::ensure!(input.len() == input_size, "Conv3D: input size mismatch");
        anyhow::ensure!(
            weights.len() == weight_size,
            "Conv3D: weights size mismatch"
        );
        anyhow::ensure!(bias.len() == out_channels, "Conv3D: bias size mismatch");

        let shader_source = include_str!("../../shaders/conv3d.wgsl");

        // Create buffers
        let input_buffer = self.create_input_buffer(input, "Conv3D Input");
        let weight_buffer = self.create_input_buffer(weights, "Conv3D Weights");
        let bias_buffer = self.create_input_buffer(bias, "Conv3D Bias");
        let output_buffer = self.create_output_buffer(out_size, "Conv3D Output");
        let staging_buffer = self.create_staging_buffer(out_size, "Conv3D Staging");

        // Parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Conv3DParams {
            batch_size: u32,
            in_channels: u32,
            out_channels: u32,
            input_d: u32,
            input_h: u32,
            input_w: u32,
            output_d: u32,
            output_h: u32,
            output_w: u32,
            kernel_d: u32,
            kernel_h: u32,
            kernel_w: u32,
            stride_d: u32,
            stride_h: u32,
            stride_w: u32,
            padding_d: u32,
            padding_h: u32,
            padding_w: u32,
            dilation_d: u32,
            dilation_h: u32,
            dilation_w: u32,
            _pad: u32,
        }

        let params = Conv3DParams {
            batch_size: batch as u32,
            in_channels: in_channels as u32,
            out_channels: out_channels as u32,
            input_d: input_depth as u32,
            input_h: input_height as u32,
            input_w: input_width as u32,
            output_d: output_depth as u32,
            output_h: output_height as u32,
            output_w: output_width as u32,
            kernel_d: config.kernel_size.0 as u32,
            kernel_h: config.kernel_size.1 as u32,
            kernel_w: config.kernel_size.2 as u32,
            stride_d: config.stride.0 as u32,
            stride_h: config.stride.1 as u32,
            stride_w: config.stride.2 as u32,
            padding_d: config.padding.0 as u32,
            padding_h: config.padding.1 as u32,
            padding_w: config.padding.2 as u32,
            dilation_d: config.dilation.0 as u32,
            dilation_h: config.dilation.1 as u32,
            dilation_w: config.dilation.2 as u32,
            _pad: 0,
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Conv3D Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Conv3D Layout"),
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
            label: Some("Conv3D Bind Group"),
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

        let pipeline = self.create_simple_pipeline(shader_source, "Conv3D", &bind_group_layout);

        // Dispatch with 3D workgroups (4x4x4 workgroup size)
        let workgroup_x = output_width.div_ceil(4);
        let workgroup_y = output_height.div_ceil(4);
        let workgroup_z = output_depth.div_ceil(4);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Conv3D Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Conv3D Pass"),
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
