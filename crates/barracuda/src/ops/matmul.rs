//! MatMul operation - Matrix multiplication
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MatMulParams {
    m: u32,
    k: u32,
    n: u32,
    _padding: u32,
}

pub struct MatMul {
    lhs: Tensor,
    rhs: Tensor,
}

impl MatMul {
    pub fn new(lhs: Tensor, rhs: Tensor) -> Self {
        Self { lhs, rhs }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/matmul.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        
        // Assume lhs: [m, k], rhs: [k, n] -> output: [m, n]
        let m = self.lhs.shape()[0];
        let k = self.lhs.shape()[1];
        let n = self.rhs.shape()[1];
        let output_size = m * n;
        
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = MatMulParams {
            m: m as u32,
            k: k as u32,
            n: n as u32,
            _padding: 0,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MatMul Params"),
            size: std::mem::size_of::<MatMulParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MatMul BGL"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
            label: Some("MatMul BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("MatMul"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MatMul PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("MatMul Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MatMul Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MatMul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups_x = (m as u32 + 15) / 16;
            let workgroups_y = (n as u32 + 15) / 16;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![m, n], device.clone()))
    }
}

impl Tensor {
    pub fn matmul(self, other: &Self) -> Result<Self> {
        MatMul::new(self, other.clone()).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn matmul_cpu(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
        let mut result = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for p in 0..k {
                    sum += a[i * k + p] * b[p * n + j];
                }
                result[i * n + j] = sum;
            }
        }
        result
    }

    #[tokio::test]
    async fn test_matmul_basic() {
        let device = get_test_device().await;

        // 2x3 * 3x2 = 2x2
        let a_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        
        let a = Tensor::from_vec_on(
            a_data.clone(),
            vec![2, 3],
            device.clone()
        ).await.unwrap();
        
        let b = Tensor::from_vec_on(
            b_data.clone(),
            vec![3, 2],
            device.clone()
        ).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        assert_eq!(result.shape(), &[2, 2]);
        
        let output = result.to_vec().unwrap();
        let expected = matmul_cpu(&a_data, &b_data, 2, 3, 2);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_matmul_edge_cases() {
        let device = get_test_device().await;

        // Identity matrix
        let a_data = vec![1.0, 0.0, 0.0, 1.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![2, 2], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![2, 2], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        let output = result.to_vec().unwrap();
        let expected = matmul_cpu(&a_data, &b_data, 2, 2, 2);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }

        // Zero matrix
        let a_data = vec![0.0, 0.0, 0.0, 0.0];
        let b_data = vec![1.0, 2.0, 3.0, 4.0];
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![2, 2], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![2, 2], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        let output = result.to_vec().unwrap();
        
        for val in output.iter() {
            assert!(val.abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_matmul_boundary() {
        let device = get_test_device().await;

        // 1x1 matrices
        let a_data = vec![5.0];
        let b_data = vec![3.0];
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![1, 1], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![1, 1], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        let output = result.to_vec().unwrap();
        assert!((output[0] - 15.0).abs() < 1e-5);

        // Tall matrix: 4x2 * 2x3 = 4x3
        let a_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b_data = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![4, 2], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![2, 3], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        assert_eq!(result.shape(), &[4, 3]);
        
        let output = result.to_vec().unwrap();
        let expected = matmul_cpu(&a_data, &b_data, 4, 2, 3);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_matmul_large_tensor() {
        let device = get_test_device().await;

        // 64x32 * 32x64 = 64x64
        let m = 64;
        let k = 32;
        let n = 64;
        
        let a_data: Vec<f32> = (0..m*k).map(|i| (i as f32) * 0.01).collect();
        let b_data: Vec<f32> = (0..k*n).map(|i| (i as f32) * 0.01).collect();
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![m, k], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![k, n], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        assert_eq!(result.shape(), &[m, n]);
        
        let output = result.to_vec().unwrap();
        let expected = matmul_cpu(&a_data, &b_data, m, k, n);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-3); // Slightly relaxed for large accumulations
        }
    }

    #[tokio::test]
    async fn test_matmul_precision() {
        let device = get_test_device().await;

        // Test FP32 precision with typical values
        let a_data = vec![
            1.234, 2.345, 3.456,
            4.567, 5.678, 6.789,
        ];
        let b_data = vec![
            0.111, 0.222,
            0.333, 0.444,
            0.555, 0.666,
        ];
        
        let a = Tensor::from_vec_on(a_data.clone(), vec![2, 3], device.clone()).await.unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![3, 2], device.clone()).await.unwrap();
        
        let result = a.matmul(&b).unwrap();
        let output = result.to_vec().unwrap();
        let expected = matmul_cpu(&a_data, &b_data, 2, 3, 2);
        
        // Verify FP32 precision
        let max_error = output.iter().zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);
        
        assert!(max_error < 1e-5, "Max error: {} exceeds FP32 threshold", max_error);
    }
}
