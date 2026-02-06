//! Log Softmax - Numerically stable log of softmax - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation axis
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;
use crate::device::{DeviceCapabilities, WorkloadType};

/// Log Softmax operation
pub struct LogSoftmax {
    input: Tensor,
}

impl LogSoftmax {
    /// Create a new log softmax operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/log_softmax.wgsl")
    }

    /// Execute the log softmax operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let size: usize = shape.iter().product();

        // Assume last dimension is the feature dimension
        let feature_size = shape[shape.len() - 1];
        let batch_size = size / feature_size;

        // Create buffers
        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        let output_buffer = device.create_buffer_f32(size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            batch_size: u32,
            feature_size: u32,
        }

        let params = Params {
            batch_size: batch_size as u32,
            feature_size: feature_size as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LogSoftmax Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LogSoftmax Bind Group Layout"),
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
            label: Some("LogSoftmax Bind Group"),
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
            label: Some("LogSoftmax Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LogSoftmax Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogSoftmax Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LogSoftmax Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (batch_size as u32 + optimal_wg_size - 1) / optimal_wg_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
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
    /// Apply log softmax along last dimension
    pub fn log_softmax_wgsl(self) -> Result<Self> {
        LogSoftmax::new(self).execute()
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
    async fn test_log_softmax() {
        let device = get_test_device().await;

        let data = vec![1.0, 2.0, 3.0];
        let input = Tensor::new(data, vec![1, 3], device.clone());

        let output = input.log_softmax_wgsl().unwrap();

        assert_eq!(output.shape(), &[1, 3]);
        
        // Check that log_softmax is negative (since softmax values are < 1)
        let result = output.to_vec().unwrap();
        for &val in result.iter() {
            assert!(val < 0.0);
        }
    }

    #[tokio::test]
    async fn test_log_softmax_batch() {
        let device = get_test_device().await;

        let data = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
        ];
        let input = Tensor::new(data, vec![2, 3], device.clone());

        let output = input.log_softmax_wgsl().unwrap();

        assert_eq!(output.shape(), &[2, 3]);
        assert_eq!(output.to_vec().unwrap().len(), 6);
    }
}
