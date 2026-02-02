//! Scatter operation - Scatter writes
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Scatter {
    input: Tensor,
    indices: Vec<u32>,
    output_size: usize,
}

impl Scatter {
    pub fn new(input: Tensor, indices: Vec<u32>, output_size: usize) -> Self {
        Self {
            input,
            indices,
            output_size,
        }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/scatter.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(self.output_size)?;

        // Create indices buffer
        let indices_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scatter Indices"),
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
                    label: Some("Scatter BGL"),
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
            label: Some("Scatter BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Scatter"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Scatter PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Scatter Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Scatter Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Scatter Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (self.input.len() as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![self.output_size],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn scatter(self, indices: Vec<u32>, output_size: usize) -> Result<Self> {
        Scatter::new(self, indices, output_size).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn scatter_cpu(input: &[f32], indices: &[u32], output_size: usize) -> Vec<f32> {
        let mut result = vec![0.0; output_size];
        for (i, &idx) in indices.iter().enumerate() {
            result[idx as usize] = input[i];
        }
        result
    }

    #[tokio::test]
    async fn test_scatter_basic() {
        let device = get_test_device().await;

        let input_data = vec![10.0, 20.0, 30.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device)
            .await
            .unwrap();
        let indices = vec![1, 3, 0];
        let result = input.scatter(indices.clone(), 4).unwrap();

        let data = result.to_vec().unwrap();
        let expected = scatter_cpu(&input_data, &indices, 4);

        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_scatter_edge_cases() {
        let device = get_test_device().await;

        // Single element
        let input_data = vec![42.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1], device.clone())
            .await
            .unwrap();
        let indices = vec![2];
        let result = input.scatter(indices.clone(), 5).unwrap();
        let data = result.to_vec().unwrap();
        assert!((data[2] - 42.0).abs() < 1e-6);
        assert!(data[0].abs() < 1e-6); // Other elements should be 0

        // Sequential indices
        let input_data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let indices = vec![0, 1, 2, 3];
        let result = input.scatter(indices.clone(), 4).unwrap();
        let data = result.to_vec().unwrap();
        for (d, &orig) in data.iter().zip(input_data.iter()) {
            assert!((d - orig).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_scatter_boundary() {
        let device = get_test_device().await;

        // Scatter to first and last positions
        let input_data = vec![99.0, 88.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device.clone())
            .await
            .unwrap();
        let indices = vec![0, 9];
        let result = input.scatter(indices.clone(), 10).unwrap();
        let data = result.to_vec().unwrap();
        assert!((data[0] - 99.0).abs() < 1e-6);
        assert!((data[9] - 88.0).abs() < 1e-6);

        // Sparse scatter
        let input_data = vec![10.0, 20.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device)
            .await
            .unwrap();
        let indices = vec![2, 7];
        let result = input.scatter(indices.clone(), 10).unwrap();
        let data = result.to_vec().unwrap();
        let expected = scatter_cpu(&input_data, &indices, 10);
        for (d, e) in data.iter().zip(expected.iter()) {
            assert!((d - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_scatter_large_tensor() {
        let device = get_test_device().await;

        // 100 elements scattered into 1000
        let input_data: Vec<f32> = (0..100).map(|i| (i as f32) * 0.1).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![100], device)
            .await
            .unwrap();
        let indices: Vec<u32> = (0..100).map(|i| (i * 10) as u32).collect();
        let result = input.scatter(indices.clone(), 1000).unwrap();
        let data = result.to_vec().unwrap();
        let expected = scatter_cpu(&input_data, &indices, 1000);

        for (d, e) in data.iter().zip(expected.iter()) {
            assert!((d - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_scatter_precision() {
        let device = get_test_device().await;

        // Test FP32 precision
        let input_data = vec![1.234, 5.678, 9.012, 3.456];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let indices = vec![3, 0, 5, 2];
        let result = input.scatter(indices.clone(), 6).unwrap();
        let data = result.to_vec().unwrap();
        let expected = scatter_cpu(&input_data, &indices, 6);

        // Verify FP32 precision (scatter is direct copy, should be exact)
        let max_error = data
            .iter()
            .zip(expected.iter())
            .map(|(d, e)| (d - e).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_error < 1e-6,
            "Max error: {} exceeds threshold",
            max_error
        );
    }
}
