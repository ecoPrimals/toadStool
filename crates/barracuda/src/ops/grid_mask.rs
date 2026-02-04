//! GridMask - Grid-based masking augmentation (Chen et al.)
//!
//! Masks structured grid regions in images.
//! Prevents overfitting to spatial structures.
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

/// GridMask operation
pub struct GridMask {
    input: Tensor,
    ratio: f32,
    rotate: f32,
    grid_size: usize,
    seed: u64,
}

impl GridMask {
    /// Create a new grid mask operation
    pub fn new(
        input: Tensor,
        ratio: f32,
        rotate: f32,
        grid_size: usize,
        seed: u64,
    ) -> Result<Self> {
        let shape = input.shape();
        if shape.len() != 3 {
            return Err(crate::error::BarracudaError::invalid_op(
                "GridMask",
                format!("Expected 3D tensor (C, H, W), got {}D", shape.len()),
            ));
        }
        
        if ratio < 0.0 || ratio > 1.0 {
            return Err(crate::error::BarracudaError::invalid_op(
                "GridMask",
                format!("Ratio must be in [0, 1], got {}", ratio),
            ));
        }
        
        Ok(Self {
            input,
            ratio,
            rotate,
            grid_size,
            seed,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/grid_mask.wgsl")
    }

    /// Execute the grid mask operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        let channels = shape[0];
        let height = shape[1];
        let width = shape[2];
        
        // Compute random offsets from seed (CPU-side, deterministic)
        let offset_x = ((self.seed * 1103515245) % self.grid_size as u64) as usize;
        let offset_y = ((self.seed * 22695477) % self.grid_size as u64) as usize;
        
        let mask_size = (self.grid_size as f32 * self.ratio) as usize;
        let angle_rad = self.rotate * std::f32::consts::PI / 180.0;
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        
        let output_size = channels * height * width;

        // Create buffers
        let input_buffer = self.input.buffer();

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GridMask Output"),
            size: (output_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            channels: u32,
            height: u32,
            width: u32,
            ratio: f32,
            rotate: f32,
            grid_size: u32,
            offset_x: u32,
            offset_y: u32,
            mask_size: u32,
            cos_a: f32,
            sin_a: f32,
            cx: f32,
            cy: f32,
        }

        let params = Params {
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
            ratio: self.ratio,
            rotate: self.rotate,
            grid_size: self.grid_size as u32,
            offset_x: offset_x as u32,
            offset_y: offset_y as u32,
            mask_size: mask_size as u32,
            cos_a,
            sin_a,
            cx,
            cy,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GridMask Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GridMask Bind Group Layout"),
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
            label: Some("GridMask Bind Group"),
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
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("GridMask Shader"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GridMask Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GridMask Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GridMask Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GridMask Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups_x = (width as u32 + 15) / 16;
            let workgroups_y = (height as u32 + 15) / 16;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer(device, &output_buffer, output_size)?;

        Ok(Tensor::new(
            output_data,
            vec![channels, height, width],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply grid mask augmentation
    ///
    /// # Arguments
    ///
    /// * `ratio` - Mask ratio (0.0 to 1.0)
    /// * `rotate` - Rotation angle in degrees
    /// * `grid_size` - Size of grid cells
    /// * `seed` - Random seed for deterministic masking
    pub fn grid_mask(self, ratio: f32, rotate: f32, grid_size: usize, seed: u64) -> Result<Self> {
        GridMask::new(self, ratio, rotate, grid_size, seed)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_grid_mask_basic() {
        let dev = get_test_device().await;
        let image = vec![1.0; 3 * 224 * 224];
        let masked = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            3,
            224,
            224,
            0.6,
            15.0,
            96,
            11111,
        )
        .await
        .unwrap();
        assert_eq!(masked.len(), image.len());
        // Some pixels should be masked (set to 0)
        assert!(masked.iter().any(|&x| x == 0.0));
        assert!(masked.iter().any(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_grid_mask_edge_cases() {
        let dev = get_test_device().await;

        // Ratio = 0 (no masking)
        let image = vec![1.0; 1 * 32 * 32];
        let masked = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            32,
            32,
            0.0,
            0.0,
            16,
            12345,
        )
        .await
        .unwrap();
        assert_eq!(masked, image); // No masking applied

        // Small image
        let image = vec![1.0; 1 * 8 * 8];
        let masked = grid_mask(&dev.device, &dev.queue, &image, 1, 8, 8, 0.5, 0.0, 4, 99999)
            .await
            .unwrap();
        assert_eq!(masked.len(), 64);
    }

    #[tokio::test]
    async fn test_grid_mask_boundary() {
        let dev = get_test_device().await;

        // Ratio = 1.0 (maximum masking)
        let image = vec![1.0; 1 * 64 * 64];
        let masked = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            64,
            64,
            1.0,
            0.0,
            32,
            77777,
        )
        .await
        .unwrap();
        assert_eq!(masked.len(), image.len());
        assert!(masked.iter().any(|&x| x == 0.0));

        // With rotation
        let image = vec![1.0; 1 * 64 * 64];
        let masked = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            64,
            64,
            0.5,
            45.0,
            16,
            55555,
        )
        .await
        .unwrap();
        assert_eq!(masked.len(), image.len());
    }

    #[tokio::test]
    async fn test_grid_mask_large_batch() {
        let dev = get_test_device().await;

        // RGB image (3 channels)
        let channels = 3;
        let height = 128;
        let width = 128;
        let image = vec![1.0; channels * height * width];
        let masked = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            channels,
            height,
            width,
            0.6,
            30.0,
            48,
            88888,
        )
        .await
        .unwrap();
        assert_eq!(masked.len(), image.len());
        assert!(masked.iter().any(|&x| x == 0.0));
        assert!(masked.iter().any(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_grid_mask_precision() {
        let dev = get_test_device().await;

        // Deterministic with same seed
        let image = vec![1.0; 1 * 32 * 32];
        let masked1 = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            32,
            32,
            0.5,
            0.0,
            16,
            12345,
        )
        .await
        .unwrap();
        let masked2 = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            32,
            32,
            0.5,
            0.0,
            16,
            12345,
        )
        .await
        .unwrap();

        // Same seed should produce same mask
        assert_eq!(masked1, masked2);

        // Different seed should produce different mask
        let masked3 = grid_mask(
            &dev.device,
            &dev.queue,
            &image,
            1,
            32,
            32,
            0.5,
            0.0,
            16,
            99999,
        )
        .await
        .unwrap();
        let different = masked1
            .iter()
            .zip(masked3.iter())
            .any(|(a, b)| (a - b).abs() > 0.1);
        assert!(different);
    }
}
