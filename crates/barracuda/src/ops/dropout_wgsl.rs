//! Dropout - Random dropout for regularization - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its probability and seed
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Dropout operation - Random dropout for regularization
pub struct Dropout {
    input: Tensor,
    probability: f32,
    seed: u32,
}

impl Dropout {
    /// Create a new dropout operation
    pub fn new(input: Tensor, probability: f32, seed: u32) -> Self {
        Self {
            input,
            probability,
            seed,
        }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/dropout/dropout.wgsl")
    }

    /// Execute the dropout operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size: usize = self.input.shape().iter().product();

        // Calculate scale factor (1 / (1 - p)) for inverted dropout
        let scale = if self.probability < 1.0 {
            1.0 / (1.0 - self.probability)
        } else {
            0.0
        };

        // Create buffers
        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        let output_buffer = device.create_buffer_f32(size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            probability: f32,
            scale: f32,
            seed: u32,
        }

        let params = Params {
            size: size as u32,
            probability: self.probability,
            scale,
            seed: self.seed,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Dropout Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Dropout Bind Group Layout"),
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
            label: Some("Dropout Bind Group"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Dropout Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Dropout Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Dropout Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Dropout Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Convolution);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer(device, &output_buffer, size)?;

        Ok(Tensor::new(
            output_data,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply dropout with given probability
    ///
    /// # Arguments
    ///
    /// * `probability` - Probability of dropping each element (0.0 to 1.0)
    /// * `seed` - Random seed for reproducibility
    pub fn dropout_wgsl(self, probability: f32, seed: u32) -> Result<Self> {
        Dropout::new(self, probability, seed).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_dropout_deterministic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0; 100];
        let input = Tensor::new(data, vec![100], device.clone());

        let output1 = input.clone().dropout_wgsl(0.5, 42).unwrap();
        let output2 = input.dropout_wgsl(0.5, 42).unwrap();

        // Same seed should produce same result
        assert_eq!(output1.to_vec().unwrap(), output2.to_vec().unwrap());
    }

    #[tokio::test]
    async fn test_dropout_zero_probability() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 2.0, 3.0];
        let input = Tensor::new(data.clone(), vec![3], device.clone());

        let output = input.dropout_wgsl(0.0, 42).unwrap();

        // With p=0, all values should be preserved (scaled by 1.0)
        let result = output.to_vec().unwrap();
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
    }

    #[tokio::test]
    async fn test_dropout_full_probability() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 2.0, 3.0];
        let input = Tensor::new(data, vec![3], device.clone());

        let output = input.dropout_wgsl(1.0, 42).unwrap();

        // With p=1.0, all values should be zero
        let result = output.to_vec().unwrap();
        assert_eq!(result[0], 0.0);
        assert_eq!(result[1], 0.0);
        assert_eq!(result[2], 0.0);
    }
}
