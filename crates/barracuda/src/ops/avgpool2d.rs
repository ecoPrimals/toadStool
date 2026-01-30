//! AvgPool2D operation - Average pooling for 2D tensors
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AvgPool2DParams {
    input_width: u32,
    input_height: u32,
    pool_size: u32,
    stride: u32,
}

pub struct AvgPool2D {
    input: Tensor,
    pool_size: usize,
    stride: usize,
}

impl AvgPool2D {
    pub fn new(input: Tensor, pool_size: usize, stride: usize) -> Self {
        Self { input, pool_size, stride }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/avgpool2d.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        
        // Assume input shape is [height, width]
        let input_height = self.input.shape()[0];
        let input_width = self.input.shape()[1];
        let output_height = input_height / self.stride;
        let output_width = input_width / self.stride;
        let output_size = output_height * output_width;
        
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = AvgPool2DParams {
            input_width: input_width as u32,
            input_height: input_height as u32,
            pool_size: self.pool_size as u32,
            stride: self.stride as u32,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AvgPool2D Params"),
            size: std::mem::size_of::<AvgPool2DParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AvgPool2D BGL"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AvgPool2D BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("AvgPool2D"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("AvgPool2D PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("AvgPool2D Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("AvgPool2D Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AvgPool2D Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups_x = (output_width as u32 + 15) / 16;
            let workgroups_y = (output_height as u32 + 15) / 16;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![output_height, output_width], device.clone()))
    }
}

impl Tensor {
    pub fn avgpool2d(self, pool_size: usize, stride: usize) -> Result<Self> {
        AvgPool2D::new(self, pool_size, stride).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_avgpool2d_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // 4x4 input, 2x2 pool with stride 2 -> 2x2 output
        let input = Tensor::from_vec_on(
            vec![
                1.0, 2.0, 3.0, 4.0,
                5.0, 6.0, 7.0, 8.0,
                9.0, 10.0, 11.0, 12.0,
                13.0, 14.0, 15.0, 16.0,
            ],
            vec![4, 4],
            device
        ).await.unwrap();
        
        let result = input.avgpool2d(2, 2).unwrap();
        assert_eq!(result.shape(), &[2, 2]);
    }
}
