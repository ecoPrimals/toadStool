//! Sigmoid activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: σ(x) = 1 / (1 + e^(-x))

use crate::tensor::Tensor;
use crate::error::Result;

/// Sigmoid activation operation
pub struct Sigmoid {
    input: Tensor,
}

impl Sigmoid {
    /// Create Sigmoid operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/sigmoid.wgsl")
    }

    /// Execute Sigmoid on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sigmoid Bind Group Layout"),
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
            ],
        });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sigmoid Bind Group"),
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
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Sigmoid"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sigmoid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Sigmoid Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sigmoid Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sigmoid Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience method on Tensor
impl Tensor {
    /// Apply Sigmoid activation
    pub fn sigmoid(self) -> Result<Self> {
        Sigmoid::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_sigmoid_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = input.sigmoid().unwrap();
        let result = output.to_vec().unwrap();

        // Sigmoid properties: σ(0) = 0.5, σ(x) ∈ (0,1), σ(-x) = 1 - σ(x)
        assert!((result[2] - 0.5).abs() < 1e-5);
        assert!(result.iter().all(|&x| x > 0.0 && x < 1.0));
        assert!((result[0] + result[4] - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_sigmoid_edge_cases() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![-100.0, -10.0, -1e-6, 0.0, 1e-6, 10.0, 100.0],
            vec![7],
            device,
        )
        .await
        .unwrap();

        let output = input.sigmoid().unwrap();
        let result = output.to_vec().unwrap();

        assert!(result[0] < 1e-20); // σ(-100) ≈ 0
        assert!(result[1] < 1e-3);  // σ(-10) ≈ 0
        assert!((result[3] - 0.5).abs() < 1e-5); // σ(0) = 0.5
        assert!(result[5] > 0.999); // σ(10) ≈ 1
        assert!(result[6] > 0.999); // σ(100) ≈ 1
    }

    #[tokio::test]
    async fn test_sigmoid_boundary() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = input.sigmoid().unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result[0], 0.0); // σ(-inf) = 0
        assert_eq!(result[4], 1.0); // σ(+inf) = 1
        assert!(result.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_sigmoid_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) / 100.0 - 5.0).collect();
        
        let input = Tensor::from_vec_on(input_data, vec![size], device).await.unwrap();
        let output = input.sigmoid().unwrap();
        let result = output.to_vec().unwrap();

        // All in (0, 1) and monotonic
        for i in 0..result.len() {
            assert!(result[i] > 0.0 && result[i] < 1.0);
            if i > 0 {
                assert!(result[i] >= result[i-1]);
            }
        }
    }

    #[tokio::test]
    async fn test_sigmoid_precision() {
        let device = get_test_device().await;

        let input_data = vec![-5.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![9], device).await.unwrap();
        let output = input.sigmoid().unwrap();
        let gpu_result = output.to_vec().unwrap();

        // CPU reference
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();

        for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
            assert!((gpu - cpu).abs() < 1e-5, "Error at {}: GPU={}, CPU={}", i, gpu, cpu);
        }
    }
}
