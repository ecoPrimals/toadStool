//! LayerNorm operation - Layer normalization (transformer essential)
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LayerNormParams {
    epsilon: f32,
    _padding: [f32; 7], // vec3 in WGSL aligns to 16 bytes, so we need 28 bytes padding
}

pub struct LayerNorm {
    input: Tensor,
    epsilon: f32,
}

impl LayerNorm {
    pub fn new(input: Tensor, epsilon: f32) -> Self {
        Self { input, epsilon }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/layer_norm.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let params = LayerNormParams {
            epsilon: self.epsilon,
            _padding: [0.0; 7],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerNorm Params"),
            size: std::mem::size_of::<LayerNormParams>() as u64,
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
                    label: Some("LayerNorm BGL"),
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
            label: Some("LayerNorm BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("LayerNorm"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("LayerNorm PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("LayerNorm Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LayerNorm Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LayerNorm Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply Layer Normalization
    ///
    /// **Phase 3**: Now supports NPU routing!
    pub fn layer_norm(self, epsilon: f32) -> Result<Self> {
        // Phase 3: Check if NPU should be used
        if crate::ops::npu_bridge::should_route_to_npu(&self, None) {
            log::debug!("Routing layer_norm to NPU");
            return self.layer_norm_npu(epsilon);
        }
        
        // Existing WGSL path
        log::debug!("Routing layer_norm to WGSL");
        LayerNorm::new(self, epsilon).execute()
    }
    
    /// Execute Layer Normalization on NPU
    fn layer_norm_npu(&self, epsilon: f32) -> Result<Self> {
        use crate::ops::npu_bridge::{tensor_to_npu_data, npu_data_to_tensor};
        use crate::npu::ops::layer_norm::npu_layer_norm;
        
        let data = tensor_to_npu_data(self)?;
        
        // Create default gamma (scale) and beta (shift) parameters
        // gamma = all 1.0 (no scaling), beta = all 0.0 (no shift)
        let gamma = vec![1.0; data.len()];
        let beta = vec![0.0; data.len()];
        
        let result_data = npu_layer_norm(&data, &gamma, &beta, epsilon)?;
        
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

    fn layer_norm_cpu(input: &[f32], epsilon: f32) -> Vec<f32> {
        let n = input.len() as f32;
        let mean: f32 = input.iter().sum::<f32>() / n;
        let variance: f32 = input.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
        let std = (variance + epsilon).sqrt();
        input.iter().map(|x| (x - mean) / std).collect()
    }

    #[tokio::test]
    async fn test_layer_norm_basic() {
        let device = get_test_device().await;

        let input_data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();

        let data = result.to_vec().unwrap();
        assert_eq!(data.len(), 4);

        let expected = layer_norm_cpu(&input_data, 1e-5);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-4);
        }
    }

    #[tokio::test]
    async fn test_layer_norm_edge_cases() {
        let device = get_test_device().await;

        // All same values (zero variance)
        let input_data = vec![3.0, 3.0, 3.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone())
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();
        let data = result.to_vec().unwrap();
        // Should be all zeros (normalized to mean)
        for val in data.iter() {
            assert!(val.abs() < 1e-3);
        }

        // Negative and positive mix
        let input_data = vec![-3.0, -1.0, 1.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone())
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();
        let data = result.to_vec().unwrap();
        let expected = layer_norm_cpu(&input_data, 1e-5);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-4);
        }
    }

    #[tokio::test]
    async fn test_layer_norm_boundary() {
        let device = get_test_device().await;

        // Single element
        let input_data = vec![7.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1], device.clone())
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();
        let data = result.to_vec().unwrap();
        assert!(data[0].abs() < 1e-3); // Should be ~0

        // Large variance
        let input_data = vec![-1000.0, -500.0, 0.0, 500.0, 1000.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device.clone())
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();
        let data = result.to_vec().unwrap();
        let expected = layer_norm_cpu(&input_data, 1e-5);
        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-3);
        }
    }

    #[tokio::test]
    async fn test_layer_norm_large_tensor() {
        let device = get_test_device().await;

        // 512 elements (typical transformer hidden size)
        let input_data: Vec<f32> = (0..512).map(|i| (i as f32 - 256.0) * 0.01).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![512], device)
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();

        let data = result.to_vec().unwrap();
        let expected = layer_norm_cpu(&input_data, 1e-5);

        for (r, e) in data.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-3);
        }
    }

    #[tokio::test]
    async fn test_layer_norm_precision() {
        let device = get_test_device().await;

        // Test FP32 precision with transformer-typical values
        let input_data = vec![0.123, -0.456, 0.789, -0.234, 0.567, -0.890, 0.345, -0.678];
        let input = Tensor::from_vec_on(input_data.clone(), vec![8], device)
            .await
            .unwrap();
        let result = input.layer_norm(1e-5).unwrap();

        let data = result.to_vec().unwrap();
        let expected = layer_norm_cpu(&input_data, 1e-5);

        // Verify FP32 precision
        let max_error = data
            .iter()
            .zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_error < 1e-4,
            "Max error: {} exceeds FP32 threshold",
            max_error
        );
    }
}
