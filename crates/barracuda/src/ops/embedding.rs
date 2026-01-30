//! Embedding operation - Lookup table
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct EmbeddingParams {
    embedding_dim: u32,
    _padding: [u32; 7], // vec3 alignment requires 28 bytes padding
}

pub struct Embedding {
    embeddings: Tensor,
    indices: Vec<u32>,
}

impl Embedding {
    pub fn new(embeddings: Tensor, indices: Vec<u32>) -> Self {
        Self { embeddings, indices }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/embedding.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.embeddings.device();
        let embedding_dim = self.embeddings.shape()[1];
        let num_indices = self.indices.len();
        let output_size = num_indices * embedding_dim;
        
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create indices buffer
        let indices_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Embedding Indices"),
            size: (self.indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&indices_buffer, 0, bytemuck::cast_slice(&self.indices));

        let params = EmbeddingParams {
            embedding_dim: embedding_dim as u32,
            _padding: [0; 7],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Embedding Params"),
            size: std::mem::size_of::<EmbeddingParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Embedding BGL"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Embedding BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.embeddings.buffer().as_entire_binding(),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Embedding"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Embedding PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Embedding Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Embedding Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Embedding Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (num_indices as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![num_indices, embedding_dim], device.clone()))
    }
}

impl Tensor {
    pub fn embedding(self, indices: Vec<u32>) -> Result<Self> {
        Embedding::new(self, indices).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_embedding_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // 3 embeddings, each of dim 4
        let embeddings = Tensor::from_vec_on(
            vec![
                1.0, 2.0, 3.0, 4.0,   // embedding 0
                5.0, 6.0, 7.0, 8.0,   // embedding 1
                9.0, 10.0, 11.0, 12.0, // embedding 2
            ],
            vec![3, 4],
            device
        ).await.unwrap();
        
        let result = embeddings.embedding(vec![1, 0, 2]).unwrap();
        assert_eq!(result.shape(), &[3, 4]);
    }
}
