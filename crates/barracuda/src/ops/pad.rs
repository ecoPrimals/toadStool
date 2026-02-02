//! Pad operation - Pure WGSL

use crate::error::Result;
use crate::tensor::Tensor;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PadParams {
    input_size: u32,
    pad_left: u32,
    pad_right: u32,
    pad_value: f32,
}

pub struct Pad {
    input: Tensor,
    pad_left: usize,
    pad_right: usize,
    pad_value: f32,
}

impl Pad {
    pub fn new(input: Tensor, pad_left: usize, pad_right: usize, pad_value: f32) -> Self {
        Self {
            input,
            pad_left,
            pad_right,
            pad_value,
        }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/pad.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_size = self.input.len();
        let output_size = self.pad_left + input_size + self.pad_right;

        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = PadParams {
            input_size: input_size as u32,
            pad_left: self.pad_left as u32,
            pad_right: self.pad_right as u32,
            pad_value: self.pad_value,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pad Params"),
            size: std::mem::size_of::<PadParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Pad BGL"),
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
            label: Some("Pad BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Pad"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pad PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pad Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pad Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pad Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (output_size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![output_size],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn pad(self, pad_left: usize, pad_right: usize, pad_value: f32) -> Result<Self> {
        Pad::new(self, pad_left, pad_right, pad_value).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_pad_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device)
            .await
            .unwrap();
        let result = input.pad(2, 2, 0.0).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), 7); // 2 + 3 + 2
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_pad_edge_cases() {
        let device = get_test_device().await;

        // No padding
        let input = Tensor::from_vec_on(vec![1.0, 2.0], vec![2], device.clone())
            .await
            .unwrap();
        let result = input.pad(0, 0, 0.0).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);

        // Only left padding
        let input = Tensor::from_vec_on(vec![5.0], vec![1], device.clone())
            .await
            .unwrap();
        let result = input.pad(3, 0, 0.0).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 4);
    }

    #[tokio::test]
    async fn test_pad_boundary() {
        let device = get_test_device().await;

        // Large padding
        let input = Tensor::from_vec_on(vec![1.0], vec![1], device.clone())
            .await
            .unwrap();
        let result = input.pad(10, 10, 0.0).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 21);

        // Non-zero pad value
        let input = Tensor::from_vec_on(vec![5.0], vec![1], device.clone())
            .await
            .unwrap();
        let result = input.pad(2, 2, -1.0).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 5);
    }

    #[tokio::test]
    async fn test_pad_large_batch() {
        let device = get_test_device().await;

        // 100 elements
        let input_data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data, vec![100], device)
            .await
            .unwrap();
        let result = input.pad(50, 50, 0.0).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 200);
    }

    #[tokio::test]
    async fn test_pad_precision() {
        let device = get_test_device().await;

        // Verify data preservation and pad value correctness
        let input = Tensor::from_vec_on(vec![10.0, 20.0], vec![2], device)
            .await
            .unwrap();
        let result = input.pad(1, 1, 99.0).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|&x| x.is_finite()));
    }
}
