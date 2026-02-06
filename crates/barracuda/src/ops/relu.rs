//! ReLU (Rectified Linear Unit) activation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (universal compute)
//! - ✅ Capability-based dispatch (vendor-optimized)
//!
//! Formula: ReLU(x) = max(0, x)

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

/// ReLU activation operation
pub struct ReLU {
    input: Tensor,
}

impl ReLU {
    /// Create ReLU operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/relu.wgsl")
    }

    /// Execute ReLU on tensor
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
                    label: Some("ReLU Bind Group Layout"),
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
            label: Some("ReLU Bind Group"),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("ReLU"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ReLU Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ReLU Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ReLU Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ReLU Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
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
    /// Apply ReLU activation: max(0, x)
    ///
    /// **Phase 3**: Now supports NPU routing!
    pub fn relu(self) -> Result<Self> {
        // Phase 3: Check if NPU should be used
        if crate::ops::npu_bridge::should_route_to_npu(&self, None) {
            log::debug!("Routing relu to NPU");
            return self.relu_npu();
        }

        // Existing WGSL path
        log::debug!("Routing relu to WGSL");
        ReLU::new(self).execute()
    }

    /// Execute ReLU on NPU
    fn relu_npu(&self) -> Result<Self> {
        use crate::npu::ops::relu::npu_relu;
        use crate::ops::npu_bridge::{npu_data_to_tensor, tensor_to_npu_data};

        let data = tensor_to_npu_data(self)?;

        // ReLU doesn't need NPU backend, it's pure computation
        let result_data = npu_relu(&data)?;

        let device = self.device().clone();
        let shape = self.shape().to_vec();

        futures::executor::block_on(npu_data_to_tensor(result_data, shape, device))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_relu_basic() {
        let device = get_test_device().await;

        // Test data: [-2, -1, 0, 1, 2]
        let input = Tensor::from_vec_on(vec![-2.0, -1.0, 0.0, 1.0, 2.0], vec![5], device)
            .await
            .unwrap();

        let output = input.relu().unwrap();
        let result = output.to_vec().unwrap();

        // Expected: [0, 0, 0, 1, 2]
        assert!((result[0] - 0.0).abs() < 1e-5);
        assert!((result[1] - 0.0).abs() < 1e-5);
        assert!((result[2] - 0.0).abs() < 1e-5);
        assert!((result[3] - 1.0).abs() < 1e-5);
        assert!((result[4] - 2.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_relu_edge_cases() {
        // Edge cases: very small values near zero
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-1e-6, -1e-10, 0.0, 1e-10, 1e-6], vec![5], device)
            .await
            .unwrap();

        let output = input.relu().unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result[0], 0.0); // Small negative → 0
        assert_eq!(result[1], 0.0); // Tiny negative → 0
        assert_eq!(result[2], 0.0); // Zero → 0
        assert!(result[3] >= 0.0); // Tiny positive → positive
        assert!(result[4] > 0.0); // Small positive → positive
    }

    #[tokio::test]
    async fn test_relu_boundary() {
        // Boundary: infinities and large values
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = input.relu().unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result[0], 0.0); // -inf → 0
        assert_eq!(result[1], 0.0); // Large negative → 0
        assert_eq!(result[2], 0.0); // 0 → 0
        assert_eq!(result[3], 1e10); // Large positive → unchanged
        assert!(result[4].is_infinite() && result[4].is_sign_positive()); // +inf → +inf
    }

    #[tokio::test]
    async fn test_relu_large_tensor() {
        // Stress test: 1000 elements
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) - 500.0).collect();

        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();

        let output = input.relu().unwrap();
        let result = output.to_vec().unwrap();

        // Verify all elements correct
        for (i, &out) in result.iter().enumerate() {
            let expected = input_data[i].max(0.0);
            assert!((out - expected).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_relu_precision() {
        // Precision: GPU vs CPU reference
        let device = get_test_device().await;

        let input_data = vec![-5.0, -2.5, -1.0, -0.5, 0.0, 0.5, 1.0, 2.5, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![9], device)
            .await
            .unwrap();

        let output = input.relu().unwrap();
        let gpu_result = output.to_vec().unwrap();

        // CPU reference
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| x.max(0.0)).collect();

        // Should be exact (no numerical error in ReLU)
        for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
            assert_eq!(gpu, cpu, "Mismatch at {}: GPU={}, CPU={}", i, gpu, cpu);
        }
    }
}
