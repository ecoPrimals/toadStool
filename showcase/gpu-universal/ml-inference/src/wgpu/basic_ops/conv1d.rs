//! 1D Convolution

use anyhow::Result;
use wgpu::util::DeviceExt;

use super::super::executor::WgpuExecutor;

impl WgpuExecutor {
    pub async fn execute_conv1d(
        &self,
        input: &[f32],
        weight: &[f32],
        bias: &[f32],
        batch: usize,
        in_channels: usize,
        out_channels: usize,
        in_length: usize,
        config: super::super::types::Conv1DConfig,
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
        let out_length =
            (in_length + 2 * config.padding - config.dilation * (config.kernel_size - 1) - 1)
                / config.stride
                + 1;
        let out_size = batch * out_channels * out_length;

        let shader_source = include_str!("../../shaders/conv1d.wgsl");

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

        let params_buffer = self
            .device
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
}
