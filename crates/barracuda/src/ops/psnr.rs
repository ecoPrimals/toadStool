//! PSNR - Peak Signal-to-Noise Ratio
//!
//! Measures reconstruction quality in dB.
//! Higher is better (typically 30-50 dB for good quality).
//!
//! Deep Debt Principles:
//! - Pure GPU/WGSL execution
//! - Safe Rust wrappers
//! - Hardware-agnostic via WebGPU
//! - Runtime device discovery
//! - Zero CPU fallbacks in execution

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// PSNR operation
pub struct PSNR {
    original: Tensor,
    reconstructed: Tensor,
    max_pixel_value: f32,
}

impl PSNR {
    /// Create a new PSNR operation
    pub fn new(original: Tensor, reconstructed: Tensor, max_pixel_value: f32) -> Result<Self> {
        let shape1 = original.shape();
        let shape2 = reconstructed.shape();

        if shape1 != shape2 {
            return Err(crate::error::BarracudaError::invalid_op(
                "PSNR",
                format!("Tensors must have same shape: {:?} vs {:?}", shape1, shape2),
            ));
        }

        if original.is_empty() {
            return Err(crate::error::BarracudaError::invalid_op(
                "PSNR",
                "Empty tensors",
            ));
        }

        Ok(Self {
            original,
            reconstructed,
            max_pixel_value,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/psnr.wgsl")
    }

    /// Execute the PSNR operation
    pub fn execute(self) -> Result<f32> {
        let device = self.original.device();
        let size = self.original.len();

        // Create buffers
        let original_buffer = self.original.buffer();
        let reconstructed_buffer = self.reconstructed.buffer();

        let mse_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PSNR MSE Output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            max_pixel_value: f32,
        }

        let params = Params {
            size: size as u32,
            max_pixel_value: self.max_pixel_value,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PSNR Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("PSNR Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PSNR Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: original_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: reconstructed_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mse_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("PSNR Shader"));

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("PSNR Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("PSNR Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PSNR Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PSNR Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results and compute MSE
        let mse_data = crate::utils::read_buffer(device, &mse_buffer, size)?;
        let mse = mse_data.iter().sum::<f32>() / size as f32;

        if mse < 1e-10 {
            return Ok(f32::INFINITY); // Perfect reconstruction
        }

        // PSNR = 10 * log10(MAX^2 / MSE)
        let psnr_val = 10.0 * (self.max_pixel_value * self.max_pixel_value / mse).log10();

        Ok(psnr_val)
    }
}

impl Tensor {
    /// Compute PSNR between two tensors
    ///
    /// # Arguments
    ///
    /// * `other` - Reconstructed tensor (must have same shape)
    /// * `max_pixel_value` - Maximum pixel value (typically 1.0 or 255.0)
    pub fn psnr(self, other: Tensor, max_pixel_value: f32) -> Result<f32> {
        PSNR::new(self, other, max_pixel_value)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_psnr_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let original = Tensor::new(vec![0.5; 1000], vec![1000], device.clone());
        let reconstructed = Tensor::new(vec![0.5; 1000], vec![1000], device.clone());
        let psnr_val = original.psnr(reconstructed, 1.0).unwrap();
        assert!(psnr_val > 100.0); // Should be very high for identical signals
    }

    #[tokio::test]
    async fn test_psnr_edge_cases() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Perfect reconstruction
        let original = Tensor::new(vec![0.1, 0.5, 0.9], vec![3], device.clone());
        let reconstructed = Tensor::new(vec![0.1, 0.5, 0.9], vec![3], device.clone());
        let psnr_val = original.psnr(reconstructed, 1.0).unwrap();
        assert!(psnr_val.is_infinite()); // MSE ~= 0

        // Significant difference (low PSNR)
        let original = Tensor::new(vec![1.0; 100], vec![100], device.clone());
        let reconstructed = Tensor::new(vec![0.5; 100], vec![100], device.clone());
        let psnr_val = original.psnr(reconstructed, 1.0).unwrap();
        assert!(psnr_val.is_finite());
        assert!(psnr_val < 10.0); // Poor quality
    }
}
