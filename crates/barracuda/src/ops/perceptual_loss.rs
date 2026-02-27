//! PerceptualLoss - Feature-based perceptual loss
//!
//! **Canonical BarraCuda Pattern**: Struct with new/execute
//!
//! Compares high-level features instead of pixels.
//! Used in style transfer and super-resolution.

use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Perceptual Loss operation
pub struct PerceptualLoss {
    features1: Tensor,
    features2: Tensor,
    weights: Option<Tensor>,
}

impl PerceptualLoss {
    /// Create a new perceptual loss operation
    pub fn new(features1: Tensor, features2: Tensor, weights: Option<Tensor>) -> Result<Self> {
        // Validate feature dimensions match
        if features1.shape() != features2.shape() {
            return Err(BarracudaError::shape_mismatch(
                features1.shape().to_vec(),
                features2.shape().to_vec(),
            ));
        }

        // Validate weights if provided
        if let Some(ref w) = weights {
            let features_size: usize = features1.shape().iter().product();
            let weights_size: usize = w.shape().iter().product();
            if !features_size.is_multiple_of(weights_size) {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "Weights dimension mismatch: features size {} must be divisible by weights size {}",
                        features_size, weights_size
                    ),
                });
            }
        }

        Ok(Self {
            features1,
            features2,
            weights,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        static SHADER: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!(
                "../shaders/loss/perceptual_loss_f64.wgsl"
            ))
        });
        &SHADER
    }

    /// Execute the perceptual loss operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.features1.device();
        let size = self.features1.len();

        let num_workgroups = (size as u32).div_ceil(crate::device::capabilities::WORKGROUP_SIZE_1D);

        // Create reduction buffer (one slot per workgroup) and output buffer
        let loss_buffer = device.create_buffer_f32(num_workgroups as usize)?;
        let output_buffer = device.create_buffer_f32(1)?;
        device.write_buffer_f32(&loss_buffer, &vec![0.0; num_workgroups as usize])?;

        // Determine if weights are provided and number of weight groups
        let has_weights = self.weights.is_some() as u32;
        let num_weights = self
            .weights
            .as_ref()
            .map(|w| w.shape().iter().product::<usize>())
            .unwrap_or(0) as u32;

        // Owned dummy buffer if weights are None (keeps it alive for bind group creation)
        let dummy_buf;
        let weights_buffer = if let Some(ref w) = self.weights {
            w.buffer()
        } else {
            dummy_buf = device.create_buffer_f32(1)?;
            &dummy_buf
        };

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            has_weights: u32,
            num_weights: u32,
            num_partials: u32,
        }

        let params = Params {
            size: size as u32,
            has_weights,
            num_weights,
            num_partials: num_workgroups,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PerceptualLoss Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module =
            device.compile_shader(Self::wgsl_shader(), Some("PerceptualLoss Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("PerceptualLoss Bind Group Layout"),
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
                });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PerceptualLoss Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.features1.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.features2.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: loss_buffer.as_entire_binding(),
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

        // Create compute pipelines
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("PerceptualLoss Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline_pass1 =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("PerceptualLoss Pipeline Pass1"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_pipeline_pass2 =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("PerceptualLoss Pipeline Pass2"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "compute_mean_loss",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Execute compute shader (two passes)
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PerceptualLoss Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PerceptualLoss Pass"),
                timestamp_writes: None,
            });

            // Pass 1: Compute weighted squared differences
            compute_pass.set_pipeline(&compute_pipeline_pass1);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch using standard 1D shader workgroup size (256)
            let caps = DeviceCapabilities::from_device(device);
            let workgroups = caps.dispatch_1d(size as u32);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);

            // Pass 2: Sum partial results and compute mean (1 workgroup)
            compute_pass.set_pipeline(&compute_pipeline_pass2);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        device.submit_and_poll(Some(encoder.finish()));

        let output_data = crate::utils::read_buffer(device, &output_buffer, 1)?;
        Ok(Tensor::new(output_data, vec![1], device.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_perceptual_loss() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let features1 = Tensor::from_vec_on(vec![0.5; 1000], vec![1000], device.clone())
            .await
            .unwrap();
        let features2 = Tensor::from_vec_on(vec![0.6; 1000], vec![1000], device.clone())
            .await
            .unwrap();
        let loss = PerceptualLoss::new(features1, features2, None)
            .unwrap()
            .execute()
            .unwrap();
        let result = loss.to_vec().unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0] > 0.0);
    }
}
