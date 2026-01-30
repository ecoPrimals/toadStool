//! OneHot operation - Convert indices to one-hot encoded vectors
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct OneHotParams {
    num_classes: u32,
    _padding: [u32; 3],
}

pub struct OneHot {
    indices: Tensor,
    num_classes: usize,
}

impl OneHot {
    pub fn new(indices: Tensor, num_classes: usize) -> Self {
        Self { indices, num_classes }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/one_hot.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.indices.device();
        let num_indices = self.indices.len();
        let output_size = num_indices * self.num_classes;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = OneHotParams {
            num_classes: self.num_classes as u32,
            _padding: [0; 3],
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("OneHot Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("OneHot Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("OneHot Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("OneHot Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.indices.buffer().as_entire_binding(),
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

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("OneHot Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("OneHot Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((output_size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_indices, self.num_classes],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Convert indices to one-hot encoded vectors
    /// # Arguments
    /// * `num_classes` - Number of classes for one-hot encoding
    pub fn one_hot(self, num_classes: usize) -> Result<Self> {
        OneHot::new(self, num_classes).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_one_hot_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create indices [0, 1, 2]
        let indices_data: Vec<u32> = vec![0, 1, 2];
        let indices = Tensor::from_data(&indices_data, vec![3], device.clone()).unwrap();
        
        // One-hot encode with 3 classes
        let result = indices.one_hot(3).unwrap();
        let output = result.to_vec().unwrap();
        
        // Expected: [[1,0,0], [0,1,0], [0,0,1]]
        assert_eq!(output.len(), 9);
        assert_eq!(output[0], 1.0); // Class 0
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0); // Class 1
        assert_eq!(output[4], 1.0);
        assert_eq!(output[5], 0.0);
        assert_eq!(output[6], 0.0); // Class 2
        assert_eq!(output[7], 0.0);
        assert_eq!(output[8], 1.0);
    }
}
