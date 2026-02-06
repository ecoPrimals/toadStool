//! Softmax activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: softmax(x_i) = exp(x_i) / Σ exp(x_j)

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Softmax activation operation
pub struct Softmax {
    input: Tensor,
}

impl Softmax {
    /// Create Softmax operation
    pub fn new(input: Tensor) -> Result<Self> {
        // Softmax expects 1D or last dimension for now
        if input.shape().is_empty() {
            return Err(BarracudaError::invalid_op(
                "Softmax",
                "Empty tensor not supported",
            ));
        }
        Ok(Self { input })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/softmax_simple.wgsl")
    }

    /// Execute Softmax on tensor
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
                    label: Some("Softmax Bind Group Layout"),
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
            label: Some("Softmax Bind Group"),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Softmax"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Softmax Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Softmax Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            // Softmax is a reduction operation over the last dimension
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
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
    /// Apply Softmax activation
    ///
    /// **Phase 3**: Now supports NPU routing!
    pub fn softmax(self) -> Result<Self> {
        // Phase 3: Check if NPU should be used
        if crate::ops::npu_bridge::should_route_to_npu(&self, None) {
            log::debug!("Routing softmax to NPU");
            return self.softmax_npu();
        }

        // Existing WGSL path
        log::debug!("Routing softmax to WGSL");
        Softmax::new(self)?.execute()
    }

    /// Execute Softmax on NPU
    fn softmax_npu(&self) -> Result<Self> {
        use crate::npu::ops::softmax::npu_softmax;
        use crate::ops::npu_bridge::{npu_data_to_tensor, tensor_to_npu_data};

        let data = tensor_to_npu_data(self)?;

        // Use default temperature of 1.0
        let result_data = npu_softmax(&data, 1.0)?;

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
    async fn test_softmax_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device)
            .await
            .unwrap();
        let output = input.softmax().unwrap();
        let result = output.to_vec().unwrap();

        // Sum = 1, all in (0,1), monotonic
        assert!(result.iter().all(|&x| x > 0.0 && x < 1.0));
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(result[2] > result[1] && result[1] > result[0]);
    }

    #[tokio::test]
    async fn test_softmax_edge_cases() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![1e-6, 2e-6, 3e-6], vec![3], device)
            .await
            .unwrap();
        let output = input.softmax().unwrap();
        let result = output.to_vec().unwrap();

        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_softmax_boundary() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![100.0, 200.0, 300.0], vec![3], device)
            .await
            .unwrap();
        let output = input.softmax().unwrap();
        let result = output.to_vec().unwrap();

        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(result[2] > 0.99); // Largest dominates
    }

    #[tokio::test]
    async fn test_softmax_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) / 10.0).collect();
        let input = Tensor::from_vec_on(input_data, vec![size], device)
            .await
            .unwrap();
        let output = input.softmax().unwrap();
        let result = output.to_vec().unwrap();

        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_softmax_precision() {
        let device = get_test_device().await;

        fn softmax_cpu(x: &[f32]) -> Vec<f32> {
            let max = x.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let exps: Vec<f32> = x.iter().map(|&v| (v - max).exp()).collect();
            let sum: f32 = exps.iter().sum();
            exps.iter().map(|&e| e / sum).collect()
        }

        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let output = input.softmax().unwrap();
        let gpu_result = output.to_vec().unwrap();
        let cpu_result = softmax_cpu(&input_data);

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
