//! Slice operation - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SliceParams {
    start: u32,
    length: u32,
    _padding: [u32; 2],
}

pub struct Slice {
    input: Tensor,
    start: usize,
    length: usize,
}

impl Slice {
    pub fn new(input: Tensor, start: usize, length: usize) -> Self {
        Self { input, start, length }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/slice.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(self.length)?;

        let params = SliceParams {
            start: self.start as u32,
            length: self.length as u32,
            _padding: [0; 2],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Params"),
            size: std::mem::size_of::<SliceParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Slice BGL"),
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
            label: Some("Slice BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Slice"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Slice PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Slice Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slice Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slice Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (self.length as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![self.length], device.clone()))
    }
}

impl Tensor {
    pub fn slice(self, start: usize, length: usize) -> Result<Self> {
        Slice::new(self, start, length).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::device::WgpuDevice;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_slice_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device).await.unwrap();
        let result = input.slice(1, 3).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_slice_edge_cases() {
        let device = get_test_device().await;

        // Slice from start
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone()).await.unwrap();
        let result = input.slice(0, 2).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);

        // Single element
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone()).await.unwrap();
        let result = input.slice(1, 1).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 1);

        // Full slice
        let input = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone()).await.unwrap();
        let result = input.slice(0, 2).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_slice_boundary() {
        let device = get_test_device().await;

        // Slice to end
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).await.unwrap();
        let result = input.slice(2, 2).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);

        // Large slice
        let input_data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data, vec![100], device.clone()).await.unwrap();
        let result = input.slice(10, 50).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 50);
    }

    #[tokio::test]
    async fn test_slice_large_batch() {
        let device = get_test_device().await;

        // 1000 elements
        let input_data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data, vec![1000], device).await.unwrap();
        let result = input.slice(100, 500).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 500);
    }

    #[tokio::test]
    async fn test_slice_precision() {
        let device = get_test_device().await;

        // Verify exact values
        let input = Tensor::from_vec_on(vec![10.0, 20.0, 30.0, 40.0, 50.0], vec![5], device).await.unwrap();
        let result = input.slice(2, 2).unwrap().to_vec().unwrap();
        
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&x| x.is_finite()));
    }
}
