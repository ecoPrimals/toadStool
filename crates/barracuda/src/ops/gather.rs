//! Gather operation - Advanced indexing
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Gather {
    input: Tensor,
    indices: Vec<u32>,
}

impl Gather {
    pub fn new(input: Tensor, indices: Vec<u32>) -> Self {
        Self { input, indices }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/gather.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_size = self.indices.len();
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create indices buffer
        let indices_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gather Indices"),
            size: (self.indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&indices_buffer, 0, bytemuck::cast_slice(&self.indices));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Gather BGL"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gather BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Gather"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Gather PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Gather Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gather Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gather Pass"),
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
    pub fn gather(self, indices: Vec<u32>) -> Result<Self> {
        Gather::new(self, indices).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn gather_cpu(input: &[f32], indices: &[u32]) -> Vec<f32> {
        indices.iter().map(|&idx| input[idx as usize]).collect()
    }

    #[tokio::test]
    async fn test_gather_basic() {
        let device = get_test_device().await;

        let input_data = vec![10.0, 20.0, 30.0, 40.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let indices = vec![2, 0, 3];
        let result = input.gather(indices.clone()).unwrap();

        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);

        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_gather_edge_cases() {
        let device = get_test_device().await;

        // Single index
        let input_data = vec![5.0, 10.0, 15.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone())
            .await
            .unwrap();
        let indices = vec![1];
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        assert!((data[0] - 10.0).abs() < 1e-6);

        // Repeated indices
        let input_data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone())
            .await
            .unwrap();
        let indices = vec![0, 0, 2, 2];
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_gather_boundary() {
        let device = get_test_device().await;

        // Gather all elements in order
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device.clone())
            .await
            .unwrap();
        let indices = vec![0, 1, 2, 3, 4];
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }

        // Gather in reverse
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device.clone())
            .await
            .unwrap();
        let indices = vec![4, 3, 2, 1, 0];
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_gather_large_tensor() {
        let device = get_test_device().await;

        // Large input tensor, selective gathering
        let input_data: Vec<f32> = (0..1000).map(|i| (i as f32) * 0.1).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![1000], device)
            .await
            .unwrap();

        // Gather every 10th element
        let indices: Vec<u32> = (0..100).map(|i| (i * 10) as u32).collect();
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);

        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_gather_precision() {
        let device = get_test_device().await;

        // Test FP32 precision
        let input_data = vec![1.234, 5.678, 9.012, 3.456, 7.890, 2.345, 6.789];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device)
            .await
            .unwrap();
        let indices = vec![6, 2, 4, 1, 0];
        let result = input.gather(indices.clone()).unwrap();
        let data = result.to_vec().unwrap();
        let expected = gather_cpu(&input_data, &indices);

        // Verify FP32 precision (gather is direct copy, should be exact)
        let max_error = data
            .iter()
            .zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_error < 1e-6,
            "Max error: {} exceeds threshold",
            max_error
        );
    }
}
