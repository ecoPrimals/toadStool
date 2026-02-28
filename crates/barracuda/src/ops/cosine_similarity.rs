//! Cosine Similarity
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Computes cosine similarity between pairs of vectors

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CosineSimilarityParams {
    num_vectors_a: u32,
    num_vectors_b: u32,
    vector_dim: u32,
    _padding: u32,
}

pub struct CosineSimilarity {
    vectors_a: Tensor,
    vectors_b: Tensor,
}

impl CosineSimilarity {
    /// Create CosineSimilarity operation
    pub fn new(vectors_a: Tensor, vectors_b: Tensor) -> Result<Self> {
        Ok(Self {
            vectors_a,
            vectors_b,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/math/cosine_similarity.wgsl")
    }

    /// Execute CosineSimilarity on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.vectors_a.device();
        let a_shape = self.vectors_a.shape();
        let b_shape = self.vectors_b.shape();

        if a_shape.len() != 2 || b_shape.len() != 2 {
            return Err(BarracudaError::invalid_op(
                "CosineSimilarity",
                format!(
                    "vectors must be 2D [num_vectors, dim], got shapes {a_shape:?} and {b_shape:?}"
                ),
            ));
        }

        let num_vectors_a = a_shape[0];
        let num_vectors_b = b_shape[0];
        let vector_dim = a_shape[1];

        if b_shape[1] != vector_dim {
            return Err(BarracudaError::invalid_op(
                "CosineSimilarity",
                format!(
                    "vector dimensions must match: {} != {}",
                    vector_dim, b_shape[1]
                ),
            ));
        }

        // Create output buffer: [num_vectors_a, num_vectors_b]
        let output_size = num_vectors_a * num_vectors_b;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = CosineSimilarityParams {
            num_vectors_a: num_vectors_a as u32,
            num_vectors_b: num_vectors_b as u32,
            vector_dim: vector_dim as u32,
            _padding: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CosineSimilarity Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CosineSimilarity Bind Group Layout"),
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
            label: Some("CosineSimilarity Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.vectors_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.vectors_b.buffer().as_entire_binding(),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("CosineSimilarity"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("CosineSimilarity Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CosineSimilarity Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("CosineSimilarity Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CosineSimilarity Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (16x16 threads per workgroup)
            let workgroups_x = (num_vectors_b as u32).div_ceil(16);
            let workgroups_y = (num_vectors_a as u32).div_ceil(16);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.submit_and_poll(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_vectors_a, num_vectors_b],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_cosine_similarity_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let num_vectors_a = 3;
        let num_vectors_b = 4;
        let vector_dim = 5;

        let vectors_a = Tensor::from_vec_on(
            vec![1.0; num_vectors_a * vector_dim],
            vec![num_vectors_a, vector_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let vectors_b = Tensor::from_vec_on(
            vec![1.0; num_vectors_b * vector_dim],
            vec![num_vectors_b, vector_dim],
            device.clone(),
        )
        .await
        .unwrap();

        let result = CosineSimilarity::new(vectors_a, vectors_b)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[num_vectors_a, num_vectors_b]);
    }
}
