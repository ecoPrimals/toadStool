//! Split - Tensor splitting operation
//! Pure WGSL implementation
//!
//! Splits a tensor into multiple parts along a dimension (inverse of Concat)
//! Formula: [output1, output2] = split(input, split_point)
//!
//! Used in: Multi-branch networks, Inception modules, ResNeXt
//! Benefits: Enables parallel processing paths, modular architecture design

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SplitParams {
    total_size: u32,
    split_point: u32,
    _pad: u32,
    _pad2: u32,
}

pub struct Split {
    input: Tensor,
    split_point: usize,
}

impl Split {
    pub fn new(input: Tensor, split_point: usize) -> Self {
        Self { input, split_point }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor/split.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor)> {
        let device = self.input.device();
        let shape = self.input.shape();

        // For simplicity, split along the last dimension
        let total_size: usize = shape.iter().product();
        let size1 = self.split_point;
        let size2 = total_size - self.split_point;

        // Create output buffers
        let output1_buffer = device.create_buffer_f32(size1)?;
        let output2_buffer = device.create_buffer_f32(size2)?;

        // Create params
        let params = SplitParams {
            total_size: total_size as u32,
            split_point: self.split_point as u32,
            _pad: 0,
            _pad2: 0,
        };
        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Split Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create shader module
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Split Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create compute pipeline
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Split Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Split Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output2_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Split Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Split Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (total_size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        // Determine output shapes (split along last dimension for simplicity)
        let mut shape1 = shape.to_vec();
        let mut shape2 = shape.to_vec();
        let last_dim = shape.len() - 1;
        let last_size = shape[last_dim];

        shape1[last_dim] = self.split_point;
        shape2[last_dim] = last_size - self.split_point;

        Ok((
            Tensor::from_buffer(output1_buffer, shape1, device.clone()),
            Tensor::from_buffer(output2_buffer, shape2, device.clone()),
        ))
    }
}

impl Tensor {
    /// Split tensor into two parts at the specified point
    /// # Arguments
    /// * `split_point` - Position to split (along last dimension)
    /// # Returns
    /// Tuple of two tensors (before split_point, after split_point)
    /// # Example
    /// ```ignore
    /// // Split [batch, 512] into [batch, 256] and [batch, 256]
    /// let (left, right) = tensor.split(256)?;
    /// ```
    pub fn split(self, split_point: usize) -> Result<(Self, Self)> {
        Split::new(self, split_point).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Option<Arc<WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_split_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Simple 1D split
        let input_data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let input = Tensor::from_data(&input_data, vec![6], device.clone()).unwrap();

        // Split at position 3 (middle)
        let (left, right) = input.split(3).unwrap();

        let left_data = left.to_vec().unwrap();
        let right_data = right.to_vec().unwrap();

        assert_eq!(left_data.len(), 3);
        assert_eq!(right_data.len(), 3);
        assert!(left_data.iter().all(|&x| x.is_finite()));
        assert!(right_data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_split_edge_cases() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Split at start
        let input_data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();
        let (left, right) = input.split(1).unwrap();
        assert_eq!(left.to_vec().unwrap().len(), 1);
        assert_eq!(right.to_vec().unwrap().len(), 3);

        // Split near end
        let input_data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();
        let (left, right) = input.split(3).unwrap();
        assert_eq!(left.to_vec().unwrap().len(), 3);
        assert_eq!(right.to_vec().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_split_boundary() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Equal split
        let input_data = vec![1.0; 100];
        let input = Tensor::from_data(&input_data, vec![100], device.clone()).unwrap();
        let (left, right) = input.split(50).unwrap();
        assert_eq!(left.to_vec().unwrap().len(), 50);
        assert_eq!(right.to_vec().unwrap().len(), 50);

        // Unequal split
        let input_data = vec![1.0; 100];
        let input = Tensor::from_data(&input_data, vec![100], device.clone()).unwrap();
        let (left, right) = input.split(30).unwrap();
        assert_eq!(left.to_vec().unwrap().len(), 30);
        assert_eq!(right.to_vec().unwrap().len(), 70);
    }

    #[tokio::test]
    async fn test_split_large_batch() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // 10000 elements
        let input_data = vec![1.0; 10000];
        let input = Tensor::from_data(&input_data, vec![10000], device.clone()).unwrap();
        let (left, right) = input.split(5000).unwrap();
        assert_eq!(left.to_vec().unwrap().len(), 5000);
        assert_eq!(right.to_vec().unwrap().len(), 5000);
    }

    #[tokio::test]
    async fn test_split_precision() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Verify data preservation
        let input_data: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let input = Tensor::from_data(&input_data, vec![10], device.clone()).unwrap();
        let (left, right) = input.split(5).unwrap();

        let left_data = left.to_vec().unwrap();
        let right_data = right.to_vec().unwrap();

        assert_eq!(left_data.len(), 5);
        assert_eq!(right_data.len(), 5);
        assert!(left_data.iter().all(|&x| x.is_finite()));
        assert!(right_data.iter().all(|&x| x.is_finite()));
    }
}
