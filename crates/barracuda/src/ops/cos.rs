//! Cos operation - Cosine
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Cos { input: Tensor }

impl Cos {
    pub fn new(input: Tensor) -> Self { Self { input } }
    fn wgsl_shader() -> &'static str { include_str!("../shaders/cos.wgsl") }
    
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Cos BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cos BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.input.buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Cos"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cos PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Cos Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Cos Encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Cos Pass"), timestamp_writes: None });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        Ok(Tensor::from_buffer(output_buffer, self.input.shape().to_vec(), device.clone()))
    }
}

impl Tensor {
    pub fn cos(self) -> Result<Self> { Cos::new(self).execute() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_cos_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);
        let input = Tensor::from_vec_on(vec![0.0, std::f32::consts::PI], vec![2], device).await.unwrap();
        let result = input.cos().unwrap().to_vec().unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5); // cos(0) = 1
        assert!((result[1] - (-1.0)).abs() < 1e-5); // cos(π) = -1
    }
}
