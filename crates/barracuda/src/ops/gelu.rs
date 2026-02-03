//! GELU (Gaussian Error Linear Unit) activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: GELU(x) = x * Φ(x) where Φ is the cumulative distribution function of the standard normal distribution
//! Approximation: GELU(x) ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³)))

use crate::error::Result;
use crate::tensor::Tensor;

/// GELU activation operation
pub struct GELU {
    input: Tensor,
}

impl GELU {
    /// Create GELU operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/gelu.wgsl")
    }

    /// Execute GELU on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GELU Bind Group Layout"),
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
            label: Some("GELU Bind Group"),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("GELU"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("GELU Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GELU Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GELU Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GELU Pass"),
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
    /// Apply GELU activation
    ///
    /// **Phase 3**: Now supports NPU routing!
    pub fn gelu(self) -> Result<Self> {
        // Phase 3: Check if NPU should be used
        if crate::ops::npu_bridge::should_route_to_npu(&self, None) {
            log::debug!("Routing gelu to NPU");
            return self.gelu_npu();
        }
        
        // Existing WGSL path
        log::debug!("Routing gelu to WGSL");
        GELU::new(self).execute()
    }
    
    /// Execute GELU on NPU
    fn gelu_npu(&self) -> Result<Self> {
        use crate::ops::npu_bridge::{tensor_to_npu_data, npu_data_to_tensor};
        use crate::npu::ops::gelu::npu_gelu;
        
        let data = tensor_to_npu_data(self)?;
        let result_data = npu_gelu(&data)?;
        
        let device = self.device().clone();
        let shape = self.shape().to_vec();
        
        futures::executor::block_on(
            npu_data_to_tensor(result_data, shape, device)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn gelu_cpu(x: f32) -> f32 {
        const SQRT_2_OVER_PI: f32 = 0.7978845608;
        const GELU_CONSTANT: f32 = 0.044715;
        let x_cubed = x * x * x;
        let inner = SQRT_2_OVER_PI * (x + GELU_CONSTANT * x_cubed);
        0.5 * x * (1.0 + inner.tanh())
    }

    #[tokio::test]
    async fn test_gelu_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-2.0, -1.0, 0.0, 1.0, 2.0], vec![5], device)
            .await
            .unwrap();
        let output = input.gelu().unwrap();
        let result = output.to_vec().unwrap();

        // GELU(0) ≈ 0, monotonic, GELU(x) ≈ x for large positive x
        assert!(result[2].abs() < 0.01);
        assert!(result[3] > 0.8 && result[3] < 0.9);
        assert!(result[4] > 1.9);
    }

    #[tokio::test]
    async fn test_gelu_edge_cases() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![-10.0, -5.0, -1e-6, 0.0, 1e-6, 5.0, 10.0],
            vec![7],
            device,
        )
        .await
        .unwrap();
        let output = input.gelu().unwrap();
        let result = output.to_vec().unwrap();

        assert!(result[0].abs() < 1e-10); // GELU(-10) ≈ 0
        assert!(result[1].abs() < 1e-6); // GELU(-5) ≈ 0
        assert!(result[3].abs() < 0.01); // GELU(0) ≈ 0
        assert!((result[5] - 5.0).abs() < 0.01); // GELU(5) ≈ 5
        assert!((result[6] - 10.0).abs() < 0.01); // GELU(10) ≈ 10
    }

    #[tokio::test]
    async fn test_gelu_boundary() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device,
        )
        .await
        .unwrap();
        let output = input.gelu().unwrap();
        let result = output.to_vec().unwrap();

        // GELU(-inf) = 0, GELU(+inf) = +inf
        assert!(result[0].abs() < 1e-10 || result[0].is_nan()); // Could be 0 or NaN
        assert!(result[2].abs() < 0.01); // GELU(0) ≈ 0
        assert!(result[3] > 1e9 || result[3].is_infinite());
        assert!(result[4].is_infinite() && result[4].is_sign_positive() || result[4].is_nan());
    }

    #[tokio::test]
    async fn test_gelu_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) / 200.0 - 2.5).collect();
        let input = Tensor::from_vec_on(input_data, vec![size], device)
            .await
            .unwrap();
        let output = input.gelu().unwrap();
        let result = output.to_vec().unwrap();

        // Monotonically increasing
        for i in 1..result.len() {
            assert!(result[i] >= result[i - 1], "GELU not monotonic at {}", i);
        }
    }

    #[tokio::test]
    async fn test_gelu_precision() {
        let device = get_test_device().await;

        let input_data = vec![-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device)
            .await
            .unwrap();
        let output = input.gelu().unwrap();
        let gpu_result = output.to_vec().unwrap();

        let cpu_result: Vec<f32> = input_data.iter().map(|&x| gelu_cpu(x)).collect();

        for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
            assert!(
                (gpu - cpu).abs() < 1e-5,
                "Error at {}: GPU={}, CPU={}",
                i,
                gpu,
                cpu
            );
        }
    }
}
