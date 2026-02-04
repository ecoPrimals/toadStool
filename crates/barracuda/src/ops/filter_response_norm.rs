//! Filter Response Normalization (FRN) - Normalization without batch dependency
//!
//! **Canonical BarraCUDA Pattern**: Struct with new/execute
//!
//! Normalizes activations per filter, not per batch.
//! Enables single-sample inference.

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Filter Response Normalization operation
pub struct FilterResponseNorm {
    input: Tensor,
    gamma: Tensor,
    beta: Tensor,
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    epsilon: f32,
}

impl FilterResponseNorm {
    /// Create a new filter response normalization operation
    pub fn new(
        input: Tensor,
        gamma: Tensor,
        beta: Tensor,
        batch_size: usize,
        channels: usize,
        height: usize,
        width: usize,
        epsilon: f32,
    ) -> Result<Self> {
        // Validate input shape
        let input_shape = input.shape();
        let expected_size = batch_size * channels * height * width;
        if input_shape.iter().product::<usize>() != expected_size {
            return Err(BarracudaError::InvalidShape {
                expected: vec![batch_size, channels, height, width],
                actual: input_shape.to_vec(),
            });
        }

        // Validate gamma and beta shapes
        if gamma.shape() != &[channels] {
            return Err(BarracudaError::InvalidShape {
                expected: vec![channels],
                actual: gamma.shape().to_vec(),
            });
        }

        if beta.shape() != &[channels] {
            return Err(BarracudaError::InvalidShape {
                expected: vec![channels],
                actual: beta.shape().to_vec(),
            });
        }

        Ok(Self {
            input,
            gamma,
            beta,
            batch_size,
            channels,
            height,
            width,
            epsilon,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/filter_response_norm.wgsl")
    }

    /// Execute the filter response normalization operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let spatial_size = self.height * self.width;
        let total_elements = self.batch_size * self.channels * spatial_size;

        // Create reduction buffer for sum of squares
        let sum_sq_buffer = device.create_buffer_f32(self.batch_size * self.channels)?;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(total_elements)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            batch_size: u32,
            channels: u32,
            height: u32,
            width: u32,
            spatial_size: u32,
            epsilon: f32,
            _pad1: u32,
        }

        let params = Params {
            batch_size: self.batch_size as u32,
            channels: self.channels as u32,
            height: self.height as u32,
            width: self.width as u32,
            spatial_size: spatial_size as u32,
            epsilon: self.epsilon,
            _pad1: 0,
        };

        let params_buffer = device.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("FRN Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("FRN Shader"));

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("FRN Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            },
        );

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FRN Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gamma.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.beta.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sum_sq_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline for first pass (sum squares)
        let pipeline_layout = device.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("FRN Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            },
        );

        let compute_pipeline_pass1 = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("FRN Pipeline Pass1"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "compute_sum_sq",
            },
        );

        let compute_pipeline_pass2 = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("FRN Pipeline Pass2"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "normalize_and_scale",
            },
        );

        // Execute compute shader (two passes)
        let mut encoder = device.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("FRN Encoder"),
            },
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FRN Pass"),
                timestamp_writes: None,
            });

            // Pass 1: Compute sum of squares
            compute_pass.set_pipeline(&compute_pipeline_pass1);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((total_elements as u32 + 255) / 256, 1, 1);

            // Pass 2: Normalize and scale
            compute_pass.set_pipeline(&compute_pipeline_pass2);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((total_elements as u32 + 255) / 256, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Output shape: same as input
        let output_shape = self.input.shape().to_vec();

        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_filter_response_norm_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0; 1 * 3 * 4 * 4], vec![1, 3, 4, 4], device.clone())
            .await
            .unwrap();
        let gamma = Tensor::from_vec_on(vec![1.0; 3], vec![3], device.clone())
            .await
            .unwrap();
        let beta = Tensor::from_vec_on(vec![0.0; 3], vec![3], device.clone())
            .await
            .unwrap();
        let output = FilterResponseNorm::new(input, gamma, beta, 1, 3, 4, 4, 1e-5)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(output.shape(), &[1, 3, 4, 4]);
        let result = output.to_vec().unwrap();
        assert_eq!(result.len(), 48);
        assert!(result.iter().all(|&x| x.is_finite()));
    }
}
