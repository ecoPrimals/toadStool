//! Where/Select operation - Conditional selection
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Where {
    condition: Tensor,
    x: Tensor,
    y: Tensor,
}

impl Where {
    pub fn new(condition: Tensor, x: Tensor, y: Tensor) -> Self {
        Self { condition, x, y }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/where_select.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.condition.device();
        let size = self.condition.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Where BGL"),
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
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Where BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.condition.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.x.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.y.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Where"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Where PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Where Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Where Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Where Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, self.condition.shape().to_vec(), device.clone()))
    }
}

impl Tensor {
    pub fn where_select(condition: Self, x: Self, y: Self) -> Result<Self> {
        Where::new(condition, x, y).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_where_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        let condition = Tensor::from_vec_on(vec![1.0, 0.0, 1.0], vec![3], device.clone()).await.unwrap();
        let x = Tensor::from_vec_on(vec![10.0, 20.0, 30.0], vec![3], device.clone()).await.unwrap();
        let y = Tensor::from_vec_on(vec![100.0, 200.0, 300.0], vec![3], device).await.unwrap();
        
        let result = Tensor::where_select(condition, x, y).unwrap().to_vec().unwrap();

        assert!((result[0] - 10.0).abs() < 1e-5);   // condition true -> x
        assert!((result[1] - 200.0).abs() < 1e-5);  // condition false -> y
        assert!((result[2] - 30.0).abs() < 1e-5);   // condition true -> x
    }
}
