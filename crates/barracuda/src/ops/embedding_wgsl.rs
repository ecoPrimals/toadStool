//! Embedding - Lookup table operation - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its embedding dimensions
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Embedding operation - Lookup embeddings from a weight matrix
pub struct Embedding {
    weight: Tensor,
    indices: Vec<usize>,
}

impl Embedding {
    /// Create a new embedding operation
    pub fn new(weight: Tensor, indices: Vec<usize>) -> Self {
        Self { weight, indices }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/embedding.wgsl")
    }

    /// Execute the embedding operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.weight.device();
        let weight_shape = self.weight.shape();

        let _num_embeddings = weight_shape[0]; // Reserved for validation
        let embedding_dim = weight_shape[1];
        let num_indices = self.indices.len();

        let output_size = num_indices * embedding_dim;

        // Create buffers
        let weight_buffer = self.weight.buffer();

        let indices_u32: Vec<u32> = self.indices.iter().map(|&x| x as u32).collect();
        let indices_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Embedding Indices"),
                contents: bytemuck::cast_slice(&indices_u32),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Embedding Output"),
            size: (output_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            num_indices: u32,
            embedding_dim: u32,
        }

        let params = Params {
            num_indices: num_indices as u32,
            embedding_dim: embedding_dim as u32,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Embedding Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Embedding Bind Group Layout"),
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
            label: Some("Embedding Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weight_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
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
                    label: Some("Embedding Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Embedding Pipeline"),
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
                label: Some("Embedding Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Embedding Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer(device, &output_buffer, output_size)?;

        Ok(Tensor::new(
            output_data,
            vec![num_indices, embedding_dim],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Lookup embeddings from weight matrix
    ///
    /// # Arguments
    ///
    /// * `indices` - Indices to lookup
    pub fn embedding_wgsl(self, indices: Vec<usize>) -> Result<Self> {
        Embedding::new(self, indices).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_embedding_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        // Weight matrix: 4 embeddings of dimension 3
        let weight_data = vec![
            1.0, 2.0, 3.0, // embedding 0
            4.0, 5.0, 6.0, // embedding 1
            7.0, 8.0, 9.0, // embedding 2
            10.0, 11.0, 12.0, // embedding 3
        ];
        let weight = Tensor::new(weight_data, vec![4, 3], device.clone());

        let indices = vec![1, 0, 3];
        let output = weight.embedding_wgsl(indices).unwrap();

        assert_eq!(output.shape(), &[3, 3]);
        let result = output.to_vec().unwrap();

        // Should get embeddings 1, 0, 3
        assert_eq!(result[0], 4.0); // embedding 1
        assert_eq!(result[1], 5.0);
        assert_eq!(result[2], 6.0);
        assert_eq!(result[3], 1.0); // embedding 0
        assert_eq!(result[4], 2.0);
        assert_eq!(result[5], 3.0);
        assert_eq!(result[6], 10.0); // embedding 3
        assert_eq!(result[7], 11.0);
        assert_eq!(result[8], 12.0);
    }
}
