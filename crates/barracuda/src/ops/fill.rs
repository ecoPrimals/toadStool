//! Fill operation - Fill tensor with constant value
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;
use crate::device::WgpuDevice;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FillParams {
    value: f32,
    _padding: [f32; 7],
}

pub struct Fill {
    shape: Vec<usize>,
    value: f32,
    device: Arc<WgpuDevice>,
}

impl Fill {
    pub fn new(shape: Vec<usize>, value: f32, device: Arc<WgpuDevice>) -> Self {
        Self { shape, value, device }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/fill.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let size: usize = self.shape.iter().product();

        // Create output buffer
        let output_buffer = self.device.create_buffer_f32(size)?;

        // Create params
        let params = FillParams {
            value: self.value,
            _padding: [0.0; 7],
        };
        let params_buffer = self.device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fill Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = self.device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fill Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = self.device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fill Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fill Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = self.device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fill Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fill Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.shape,
            self.device.clone(),
        ))
    }
}

impl Tensor {
    /// Create tensor filled with constant value
    /// # Arguments
    /// * `shape` - Shape of the tensor
    /// * `value` - Value to fill with
    /// * `device` - Device to create tensor on
    pub fn fill(shape: Vec<usize>, value: f32, device: Arc<WgpuDevice>) -> Result<Self> {
        Fill::new(shape, value, device).execute()
    }
    
    /// Fill this tensor with a constant value (in-place operation concept)
    pub fn fill_with(self, value: f32) -> Result<Self> {
        Fill::new(self.shape().to_vec(), value, self.device().clone()).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fill_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Fill [3, 4] tensor with 7.5
        let result = Tensor::fill(vec![3, 4], 7.5, device).unwrap();
        let output = result.to_vec().unwrap();
        
        // All 12 elements should be 7.5
        assert_eq!(output.len(), 12);
        for val in output.iter() {
            assert_eq!(*val, 7.5);
        }
    }
}
