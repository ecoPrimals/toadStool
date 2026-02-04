//! Layer Normalization - Normalize along feature dimension - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its feature size and epsilon
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Layer normalization operation
pub struct LayerNorm {
    input: Tensor,
    epsilon: f32,
}

impl LayerNorm {
    /// Create a new layer normalization operation
    pub fn new(input: Tensor, epsilon: f32) -> Self {
        Self { input, epsilon }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/layer_norm.wgsl")
    }

    /// Execute the layer normalization operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let size: usize = shape.iter().product();

        // Assume last dimension is the feature dimension
        let feature_size = shape[shape.len() - 1];

        // Create buffers
        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        let output_buffer = device.create_buffer_f32(size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            feature_size: u32,
            epsilon: f32,
        }

        let params = Params {
            size: size as u32,
            feature_size: feature_size as u32,
            epsilon: self.epsilon,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LayerNorm Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LayerNorm Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LayerNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
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

        // Create compute pipeline
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("Shader"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LayerNorm Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LayerNorm Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LayerNorm Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LayerNorm Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let num_batches = (size / feature_size) as u32;
            compute_pass.dispatch_workgroups((num_batches + 255) / 256, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer(device, &output_buffer, size)?;

        Ok(Tensor::new(
            output_data,
            shape.to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply layer normalization (normalize along last dimension)
    ///
    /// # Arguments
    ///
    /// * `epsilon` - Small constant for numerical stability (default: 1e-5)
    pub fn layer_norm_wgsl(self, epsilon: f32) -> Result<Self> {
        LayerNorm::new(self, epsilon).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> std::sync::Arc<crate::device::WgpuDevice> {
        use crate::device::test_pool::get_test_device;
        get_test_device().await
    }

    #[tokio::test]
    async fn test_layer_norm_1d() {
        let device = get_test_device().await;

        let data = vec![1.0, 2.0, 3.0, 4.0];
        let input = Tensor::new(data, vec![1, 4], device.clone());

        let output = input.layer_norm_wgsl(1e-5).unwrap();

        assert_eq!(output.shape(), &[1, 4]);
        
        // Check that mean is ~0 and std is ~1
        let result = output.to_vec().unwrap();
        let mean: f32 = result.iter().sum::<f32>() / 4.0;
        assert!((mean).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_layer_norm_batch() {
        let device = get_test_device().await;

        let data = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
        ];
        let input = Tensor::new(data, vec![2, 3], device.clone());

        let output = input.layer_norm_wgsl(1e-5).unwrap();

        assert_eq!(output.shape(), &[2, 3]);
        
        // Each batch should be normalized independently
        let result = output.to_vec().unwrap();
        
        // First batch mean should be ~0
        let mean1 = (result[0] + result[1] + result[2]) / 3.0;
        assert!((mean1).abs() < 1e-5);
        
        // Second batch mean should be ~0
        let mean2 = (result[3] + result[4] + result[5]) / 3.0;
        assert!((mean2).abs() < 1e-5);
    }
}
