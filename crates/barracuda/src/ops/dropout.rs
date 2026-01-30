//! Dropout operation - Regularization
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DropoutParams {
    rate: f32,
    seed: u32,
    _padding: [f32; 2],
}

pub struct Dropout {
    input: Tensor,
    rate: f32,
    seed: u32,
}

impl Dropout {
    pub fn new(input: Tensor, rate: f32, seed: u32) -> Self {
        Self { input, rate, seed }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/dropout.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let params = DropoutParams {
            rate: self.rate,
            seed: self.seed,
            _padding: [0.0; 2],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dropout Params"),
            size: std::mem::size_of::<DropoutParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Dropout BGL"),
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
            label: Some("Dropout BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Dropout"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Dropout PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Dropout Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Dropout Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Dropout Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, self.input.shape().to_vec(), device.clone()))
    }
}

impl Tensor {
    pub fn dropout(self, rate: f32, seed: u32) -> Result<Self> {
        Dropout::new(self, rate, seed).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_dropout_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device).await.unwrap();
        let result = input.dropout(0.5, 12345).unwrap();
        let data = result.to_vec().unwrap();
        
        assert_eq!(data.len(), 4);
        // With p=0.5, roughly half should be zero, rest scaled by 2.0
        let zeros = data.iter().filter(|&&x| x == 0.0).count();
        assert!(zeros > 0 && zeros < 4);
    }

    #[tokio::test]
    async fn test_dropout_edge_cases() {
        let device = get_test_device().await;

        // p=0.0 (no dropout)
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).await.unwrap();
        let result = input.dropout(0.0, 12345).unwrap();
        let data = result.to_vec().unwrap();
        
        assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0]);

        // p=1.0 (all dropped)
        let input2 = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![4], device).await.unwrap();
        let result2 = input2.dropout(1.0, 12345).unwrap();
        let data2 = result2.to_vec().unwrap();
        
        assert!(data2.iter().all(|&x| x == 0.0));
    }

    #[tokio::test]
    async fn test_dropout_boundary() {
        let device = get_test_device().await;

        // Very small p
        let input = Tensor::from_vec_on(vec![1.0; 100], vec![100], device.clone()).await.unwrap();
        let result = input.dropout(0.01, 12345).unwrap();
        let data = result.to_vec().unwrap();
        
        let zeros = data.iter().filter(|&&x| x == 0.0).count();
        assert!(zeros < 10); // Should be ~1% zeros

        // Very large p
        let input2 = Tensor::from_vec_on(vec![1.0; 100], vec![100], device).await.unwrap();
        let result2 = input2.dropout(0.99, 12345).unwrap();
        let data2 = result2.to_vec().unwrap();
        
        let zeros2 = data2.iter().filter(|&&x| x == 0.0).count();
        assert!(zeros2 > 90); // Should be ~99% zeros
    }

    #[tokio::test]
    async fn test_dropout_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data = vec![1.0; size];
        let input = Tensor::from_vec_on(input_data, vec![size], device).await.unwrap();
        let result = input.dropout(0.5, 12345).unwrap();
        let data = result.to_vec().unwrap();

        let zeros = data.iter().filter(|&&x| x == 0.0).count();
        // Should be roughly 50% ± 10%
        assert!(zeros > 400 && zeros < 600);
    }

    #[tokio::test]
    async fn test_dropout_precision() {
        let device = get_test_device().await;

        // Verify scaling: kept values should be scaled by 1/(1-p)
        let input_data = vec![2.0; 100];
        let input = Tensor::from_vec_on(input_data, vec![100], device).await.unwrap();
        let result = input.dropout(0.5, 12345).unwrap();
        let data = result.to_vec().unwrap();

        // Non-zero values should be 2.0 * (1/0.5) = 4.0
        for &val in data.iter() {
            assert!(val == 0.0 || (val - 4.0).abs() < 1e-5);
        }
    }
}
