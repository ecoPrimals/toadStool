//! Lt operation - Less than comparison  
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Lt { lhs: Tensor, rhs: Tensor }

impl Lt {
    pub fn new(lhs: Tensor, rhs: Tensor) -> Self { Self { lhs, rhs } }
    fn wgsl_shader() -> &'static str { include_str!("../shaders/lt.wgsl") }
    
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Lt BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lt BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.lhs.buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.rhs.buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Lt"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Lt PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Lt Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Lt Encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Lt Pass"), timestamp_writes: None });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        Ok(Tensor::from_buffer(output_buffer, self.lhs.shape().to_vec(), device.clone()))
    }
}

impl Tensor {
    pub fn lt(self, other: &Self) -> Result<Self> { Lt::new(self, other.clone()).execute() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_lt_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);
        let a = Tensor::from_vec_on(vec![1.0, 3.0, 2.0], vec![3], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(vec![2.0, 2.0, 2.0], vec![3], device).await.unwrap();
        let result = a.lt(&b).unwrap().to_vec().unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5); // 1 < 2? yes
    }
}
