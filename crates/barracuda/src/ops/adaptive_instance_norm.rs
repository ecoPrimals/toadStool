//! Adaptive Instance Normalization (AdaIN) - Style transfer
//!
//! Transfers style from one image to another.
//! Used in neural style transfer, GANs.
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

/// AdaptiveInstanceNorm operation
pub struct AdaptiveInstanceNorm {
    content: Tensor,
    style_mean: Tensor,
    style_std: Tensor,
}

impl AdaptiveInstanceNorm {
    /// Create a new adaptive instance norm operation
    pub fn new(content: Tensor, style_mean: Tensor, style_std: Tensor) -> Result<Self> {
        let content_shape = content.shape();
        let style_mean_shape = style_mean.shape();
        let style_std_shape = style_std.shape();
        
        if content_shape.len() != 4 {
            return Err(crate::error::BarracudaError::invalid_op(
                "AdaptiveInstanceNorm",
                format!("Content must be 4D (NCHW), got {}D", content_shape.len()),
            ));
        }
        
        if style_mean_shape.len() != 1 || style_std_shape.len() != 1 {
            return Err(crate::error::BarracudaError::invalid_op(
                "AdaptiveInstanceNorm",
                "Style mean and std must be 1D tensors",
            ));
        }
        
        if style_mean_shape[0] != content_shape[1] || style_std_shape[0] != content_shape[1] {
            return Err(crate::error::BarracudaError::invalid_op(
                "AdaptiveInstanceNorm",
                "Style statistics must match number of channels",
            ));
        }
        
        Ok(Self {
            content,
            style_mean,
            style_std,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/adaptive_instance_norm.wgsl")
    }

    /// Execute the adaptive instance norm operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.content.device();
        let shape = self.content.shape();
        
        let batch = shape[0];
        let channels = shape[1];
        let height = shape[2];
        let width = shape[3];
        let spatial_size = height * width;
        let output_size = batch * channels * spatial_size;

        // Create buffers
        let content_buffer = self.content.buffer();
        let style_mean_buffer = self.style_mean.buffer();
        let style_std_buffer = self.style_std.buffer();

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AdaptiveInstanceNorm Output"),
            size: (output_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            batch: u32,
            channels: u32,
            height: u32,
            width: u32,
            spatial_size: u32,
        }

        let params = Params {
            batch: batch as u32,
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
            spatial_size: spatial_size as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("AdaptiveInstanceNorm Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AdaptiveInstanceNorm Bind Group Layout"),
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
            label: Some("AdaptiveInstanceNorm Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: content_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: style_mean_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: style_std_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("AdaptiveInstanceNorm Shader"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("AdaptiveInstanceNorm Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("AdaptiveInstanceNorm Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("AdaptiveInstanceNorm Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AdaptiveInstanceNorm Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = (output_size as u32 + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer(device, &output_buffer, output_size)?;

        Ok(Tensor::new(
            output_data,
            vec![batch, channels, height, width],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply adaptive instance normalization (AdaIN) for style transfer
    ///
    /// # Arguments
    ///
    /// * `style_mean` - Style mean tensor [C]
    /// * `style_std` - Style std tensor [C]
    pub fn adaptive_instance_norm(self, style_mean: Tensor, style_std: Tensor) -> Result<Self> {
        AdaptiveInstanceNorm::new(self, style_mean, style_std)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adaptive_instance_norm_basic() {
        let dev = get_test_device().await;
        let content = vec![1.0; 1 * 3 * 4 * 4];
        let style_mean = vec![0.5, 0.5, 0.5];
        let style_std = vec![0.2, 0.2, 0.2];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            3,
            4,
            4,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), content.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_edge_cases() {
        let dev = get_test_device().await;

        // Test with zero style std (should clamp)
        let content = vec![1.0, 2.0, 3.0, 4.0];
        let style_mean = vec![0.0];
        let style_std = vec![0.0];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Test with single channel, single pixel
        let content = vec![5.0];
        let style_mean = vec![1.0];
        let style_std = vec![2.0];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            1,
            1,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_boundary() {
        let dev = get_test_device().await;

        // Test with different style statistics
        let content = vec![0.0, 1.0, 2.0, 3.0];

        // Style 1: mean=0, std=1
        let output1 = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &vec![0.0],
            &vec![1.0],
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        // Style 2: mean=10, std=5
        let output2 = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &vec![10.0],
            &vec![5.0],
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        assert!(output1.iter().all(|&x| x.is_finite()));
        assert!(output2.iter().all(|&x| x.is_finite()));
        // Different style should produce different output
        assert_ne!(output1, output2);
        // Output2 should have higher values (mean=10)
        assert!(output2.iter().sum::<f32>() > output1.iter().sum::<f32>());
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_large_batch() {
        let dev = get_test_device().await;

        // Multiple batches and channels
        let batch_size = 2;
        let channels = 4;
        let height = 8;
        let width = 8;

        let content: Vec<f32> = (0..batch_size * channels * height * width)
            .map(|i| (i % 10) as f32)
            .collect();
        let style_mean = vec![0.5, 1.0, 1.5, 2.0];
        let style_std = vec![0.1, 0.2, 0.3, 0.4];

        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            batch_size,
            channels,
            height,
            width,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), content.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_precision() {
        let dev = get_test_device().await;

        // Test with known values for style transfer
        let content = vec![
            0.0, 1.0, 2.0, 3.0, // Mean = 1.5
        ];
        let style_mean = vec![5.0]; // Target mean
        let style_std = vec![2.0]; // Target std

        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        // After AdaIN, output should have approximately the target mean
        let out_mean = output.iter().sum::<f32>() / output.len() as f32;
        assert!((out_mean - 5.0).abs() < 0.1);

        // Output should preserve relative relationships (normalized)
        assert!(output[0] < output[1]);
        assert!(output[1] < output[2]);
        assert!(output[2] < output[3]);
    }
}
