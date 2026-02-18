//! SSIM - Structural Similarity Index (Wang et al.)
//!
//! Perceptual similarity metric for images.
//! Considers luminance, contrast, and structure.
//!
//! Deep Debt Principles:
//! - Pure GPU/WGSL execution
//! - Safe Rust wrappers
//! - Hardware-agnostic via WebGPU
//! - Runtime device discovery
//! - Zero CPU fallbacks in execution

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// SSIM operation
pub struct SSIM {
    image1: Tensor,
    image2: Tensor,
    window_size: usize,
    c1: f32,
    c2: f32,
}

impl SSIM {
    /// Create a new SSIM operation
    pub fn new(
        image1: Tensor,
        image2: Tensor,
        window_size: usize,
        c1: f32,
        c2: f32,
    ) -> Result<Self> {
        let shape1 = image1.shape();
        let shape2 = image2.shape();

        if shape1 != shape2 {
            return Err(crate::error::BarracudaError::invalid_op(
                "SSIM",
                format!("Images must have same shape: {:?} vs {:?}", shape1, shape2),
            ));
        }

        if shape1.len() != 2 {
            return Err(crate::error::BarracudaError::invalid_op(
                "SSIM",
                format!("Expected 2D tensor (H, W), got {}D", shape1.len()),
            ));
        }

        if window_size == 0 || window_size > shape1[0] || window_size > shape1[1] {
            return Err(crate::error::BarracudaError::invalid_op(
                "SSIM",
                format!("Invalid window_size: {}", window_size),
            ));
        }

        Ok(Self {
            image1,
            image2,
            window_size,
            c1,
            c2,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/ssim.wgsl")
    }

    /// Execute the SSIM operation
    pub fn execute(self) -> Result<f32> {
        let device = self.image1.device();
        let shape = self.image1.shape();

        let height = shape[0];
        let width = shape[1];
        let num_windows_x = width - self.window_size + 1;
        let num_windows_y = height - self.window_size + 1;
        let num_windows = num_windows_x * num_windows_y;

        if num_windows == 0 {
            return Err(crate::error::BarracudaError::invalid_op(
                "SSIM",
                "Window size too large for image dimensions",
            ));
        }

        // Create buffers
        let image1_buffer = self.image1.buffer();
        let image2_buffer = self.image2.buffer();

        let window_ssim_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SSIM Window Output"),
            size: (num_windows * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            width: u32,
            height: u32,
            window_size: u32,
            c1: f32,
            c2: f32,
            num_windows: u32,
        }

        let params = Params {
            width: width as u32,
            height: height as u32,
            window_size: self.window_size as u32,
            c1: self.c1,
            c2: self.c2,
            num_windows: num_windows as u32,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SSIM Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SSIM Bind Group Layout"),
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
            label: Some("SSIM Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: image2_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: window_ssim_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("SSIM Shader"));

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SSIM Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SSIM Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SSIM Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SSIM Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups_x = (num_windows_x as u32).div_ceil(16);
            let workgroups_y = (num_windows_y as u32).div_ceil(16);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results and compute mean SSIM
        let window_ssim_data = crate::utils::read_buffer(device, &window_ssim_buffer, num_windows)?;
        let ssim_sum: f32 = window_ssim_data.iter().sum();
        Ok(ssim_sum / num_windows as f32)
    }
}

impl Tensor {
    /// Compute SSIM between two images
    ///
    /// # Arguments
    ///
    /// * `other` - Second image tensor (must have same shape)
    /// * `window_size` - Size of sliding window (typically 11)
    /// * `c1` - Stability constant for luminance (typically 0.01^2)
    /// * `c2` - Stability constant for contrast (typically 0.03^2)
    pub fn ssim(self, other: Tensor, window_size: usize, c1: f32, c2: f32) -> Result<f32> {
        SSIM::new(self, other, window_size, c1, c2)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_ssim_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let image1 = Tensor::new(vec![0.5; 64 * 64], vec![64, 64], device.clone());
        let image2 = Tensor::new(vec![0.5; 64 * 64], vec![64, 64], device.clone());
        let similarity = image1.ssim(image2, 11, 0.01, 0.03).unwrap();
        assert!(similarity.is_finite());
        assert!(similarity > 0.9); // Should be close to 1.0 for identical images
    }

    #[tokio::test]
    async fn test_ssim_edge_cases() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Small image
        let image1 = Tensor::new(vec![0.5; 16 * 16], vec![16, 16], device.clone());
        let image2 = Tensor::new(vec![0.5; 16 * 16], vec![16, 16], device.clone());
        let similarity = image1.ssim(image2, 5, 0.01, 0.03).unwrap();
        assert!(similarity.is_finite());

        // Different images
        let image1 = Tensor::new(vec![0.0; 32 * 32], vec![32, 32], device.clone());
        let image2 = Tensor::new(vec![1.0; 32 * 32], vec![32, 32], device.clone());
        let similarity = image1.ssim(image2, 7, 0.01, 0.03).unwrap();
        assert!(similarity < 1.0);
    }
}
