//! RReLU - Randomized Leaky ReLU - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its slope range
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// RReLU (Randomized Leaky ReLU) operation
pub struct RReLU {
    input: Tensor,
    lower: f32,
    upper: f32,
    seed: u32,
}

impl RReLU {
    /// Create a new RReLU operation
    pub fn new(input: Tensor, lower: f32, upper: f32, seed: u32) -> Self {
        Self {
            input,
            lower,
            upper,
            seed,
        }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/rrelu.wgsl")
    }

    /// Execute the RReLU operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size: usize = self.input.shape().iter().product();

        // Create buffers
        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        let output_buffer = device.create_buffer_f32(size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            lower: f32,
            upper: f32,
            seed: u32,
        }

        let params = Params {
            size: size as u32,
            lower: self.lower,
            upper: self.upper,
            seed: self.seed,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RReLU Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RReLU Bind Group Layout"),
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
            label: Some("RReLU Bind Group"),
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
            label: Some("RReLU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RReLU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RReLU Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RReLU Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
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
    /// Apply Randomized Leaky ReLU
    ///
    /// # Arguments
    ///
    /// * `lower` - Lower bound for random slope (default: 1/8)
    /// * `upper` - Upper bound for random slope (default: 1/3)
    /// * `seed` - Random seed for reproducibility
    pub fn rrelu_wgsl(self, lower: f32, upper: f32, seed: u32) -> Result<Self> {
        RReLU::new(self, lower, upper, seed).execute()
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
    async fn test_rrelu_positive() {
        let device = get_test_device().await;

        let data = vec![1.0, 2.0, 3.0];
        let input = Tensor::new(data.clone(), vec![3], device.clone());

        let output = input.rrelu_wgsl(0.125, 0.333, 42).unwrap();

        // Positive values should be unchanged
        let result = output.to_vec().unwrap();
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
    }

    #[tokio::test]
    async fn test_rrelu_negative() {
        let device = get_test_device().await;

        let data = vec![-1.0, -2.0];
        let input = Tensor::new(data, vec![2], device.clone());

        let output = input.rrelu_wgsl(0.125, 0.333, 42).unwrap();

        // Negative values should be scaled by random slope in [0.125, 0.333]
        let result = output.to_vec().unwrap();
        assert!(result[0] > -0.333 && result[0] < -0.125);
        assert!(result[1] > -0.666 && result[1] < -0.25);
    }

    #[tokio::test]
    async fn test_rrelu_deterministic() {
        let device = get_test_device().await;

        let data = vec![-1.0];
        let input1 = Tensor::new(data.clone(), vec![1], device.clone());
        let input2 = Tensor::new(data, vec![1], device.clone());

        let output1 = input1.rrelu_wgsl(0.125, 0.333, 42).unwrap();
        let output2 = input2.rrelu_wgsl(0.125, 0.333, 42).unwrap();

        // Same seed should produce same results
        assert_eq!(output1.to_vec().unwrap(), output2.to_vec().unwrap());
    }
}
