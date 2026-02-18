//! Rotary Position Embedding (RoPE) - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Applies rotation to query/key pairs based on position.
//! Encodes relative position information without absolute position embeddings.
//!
//! Reference: RoFormer (Su et al., 2021), used in GPT-Neo, LLaMA, PaLM

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Rotary Position Embedding operation
pub struct RotaryEmbedding {
    input: Tensor,
}

impl RotaryEmbedding {
    /// Create a new rotary embedding operation
    ///
    /// **Shape**: [batch, seq_len, num_heads, head_dim]
    /// **Requirement**: head_dim must be even
    pub fn new(input: Tensor) -> Result<Self> {
        // Validate shape: must be 4D
        if input.shape().len() != 4 {
            return Err(BarracudaError::shape_mismatch(
                input.shape().to_vec(),
                vec![0, 0, 0, 0],
            ));
        }

        // Validate head_dim is even (required for pairwise rotation)
        let head_dim = input.shape()[3];
        if !head_dim.is_multiple_of(2) {
            return Err(BarracudaError::invalid_op(
                "RotaryEmbedding",
                format!("head_dim ({}) must be even for pairwise rotation", head_dim),
            ));
        }

        Ok(Self { input })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/attention/rotary_embedding.wgsl")
    }

    /// Execute the rotary embedding operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let batch_size = shape[0];
        let seq_len = shape[1];
        let num_heads = shape[2];
        let head_dim = shape[3];
        let half_dim = head_dim / 2;

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create output buffer
        let output_size = self.input.len();
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            batch_size: u32,
            seq_len: u32,
            num_heads: u32,
            head_dim: u32,
            half_dim: u32,
            _padding: [u32; 3],
        }

        let params = Params {
            batch_size: batch_size as u32,
            seq_len: seq_len as u32,
            num_heads: num_heads as u32,
            head_dim: head_dim as u32,
            half_dim: half_dim as u32,
            _padding: [0; 3],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RotaryEmbedding Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module =
            device.compile_shader(Self::wgsl_shader(), Some("RotaryEmbedding Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RotaryEmbedding Bind Group Layout"),
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
            label: Some("RotaryEmbedding Bind Group"),
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
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("RotaryEmbedding Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("RotaryEmbedding Pipeline"),
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
                label: Some("RotaryEmbedding Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RotaryEmbedding Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let total = batch_size * seq_len * num_heads * half_dim;
            let workgroups = (total as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            shape.to_vec(),
            device.clone(),
        ))
    }
}

// Note: Tensor::rotary_embedding() is implemented in rope.rs to avoid duplication

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_rotary_embedding_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(vec![1.0; 4 * 2 * 8], vec![1, 4, 2, 8], device)
            .await
            .unwrap();

        let output = input.rotary_embedding().unwrap();
        assert_eq!(output.shape(), &[1, 4, 2, 8]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_rotary_embedding_edge_cases() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Single position
        let input = Tensor::from_vec_on(vec![1.0; 2 * 8], vec![1, 1, 2, 8], device.clone())
            .await
            .unwrap();
        let output = input.rotary_embedding().unwrap();
        assert_eq!(output.shape(), &[1, 1, 2, 8]);

        // Single head
        let input = Tensor::from_vec_on(vec![1.0; 4 * 8], vec![1, 4, 1, 8], device.clone())
            .await
            .unwrap();
        let output = input.rotary_embedding().unwrap();
        assert_eq!(output.shape(), &[1, 4, 1, 8]);

        // Small head dimension
        let input = Tensor::from_vec_on(vec![1.0; 2 * 2 * 4], vec![1, 2, 2, 4], device)
            .await
            .unwrap();
        let output = input.rotary_embedding().unwrap();
        assert_eq!(output.shape(), &[1, 2, 2, 4]);
    }

    #[tokio::test]
    async fn test_rotary_embedding_shape_validation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Valid: even head_dim
        let input = Tensor::from_vec_on(vec![1.0; 4 * 2 * 8], vec![1, 4, 2, 8], device.clone())
            .await
            .unwrap();
        assert!(input.rotary_embedding().is_ok());

        // Invalid: odd head_dim
        let input = Tensor::from_vec_on(vec![1.0; 4 * 2 * 7], vec![1, 4, 2, 7], device)
            .await
            .unwrap();
        assert!(input.rotary_embedding().is_err());
    }
}
