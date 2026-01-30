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
    use crate::device::test_pool::get_test_device;

    fn embedding_cpu(embeddings: &[f32], indices: &[u32], _vocab_size: usize, embed_dim: usize) -> Vec<f32> {
        let mut result = Vec::new();
        for &idx in indices {
            let start = (idx as usize) * embed_dim;
            let end = start + embed_dim;
            result.extend_from_slice(&embeddings[start..end]);
        }
        result
    }

    #[tokio::test]
    async fn test_embedding_basic() {
        let device = get_test_device().await;

        // 3 embeddings, each of dim 4
        let embed_data = vec![
            1.0, 2.0, 3.0, 4.0,   // embedding 0
            5.0, 6.0, 7.0, 8.0,   // embedding 1
            9.0, 10.0, 11.0, 12.0, // embedding 2
        ];
        
        let embeddings = Tensor::from_vec_on(
            embed_data.clone(),
            vec![3, 4],
            device
        ).await.unwrap();
        
        let indices = vec![1, 0, 2];
        let result = embeddings.embedding(indices.clone()).unwrap();
        assert_eq!(result.shape(), &[3, 4]);
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, 3, 4);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_embedding_edge_cases() {
        let device = get_test_device().await;

        // Single embedding lookup
        let embed_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![2, 3], device.clone()).await.unwrap();
        let indices = vec![0];
        let result = embeddings.embedding(indices.clone()).unwrap();
        assert_eq!(result.shape(), &[1, 3]);
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, 2, 3);
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }

        // Repeated indices
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![2, 3], device.clone()).await.unwrap();
        let indices = vec![0, 0, 1, 1];
        let result = embeddings.embedding(indices.clone()).unwrap();
        assert_eq!(result.shape(), &[4, 3]);
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, 2, 3);
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_embedding_boundary() {
        let device = get_test_device().await;

        // Minimal embedding (1 vocab, 1 dim)
        let embed_data = vec![42.0];
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![1, 1], device.clone()).await.unwrap();
        let indices = vec![0];
        let result = embeddings.embedding(indices.clone()).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output[0], 42.0);

        // Large vocabulary
        let vocab_size = 100;
        let embed_dim = 8;
        let embed_data: Vec<f32> = (0..vocab_size * embed_dim).map(|i| i as f32 * 0.1).collect();
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![vocab_size, embed_dim], device.clone()).await.unwrap();
        let indices = vec![0, 50, 99];
        let result = embeddings.embedding(indices.clone()).unwrap();
        assert_eq!(result.shape(), &[3, embed_dim]);
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, vocab_size, embed_dim);
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_embedding_large_tensor() {
        let device = get_test_device().await;

        // Transformer-scale: 10k vocab, 512 embed dim, 32 token sequence
        let vocab_size = 1000; // Reduced from 10k for test speed
        let embed_dim = 64;     // Reduced from 512 for test speed
        let seq_len = 32;
        
        let embed_data: Vec<f32> = (0..vocab_size * embed_dim).map(|i| (i as f32) * 0.001).collect();
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![vocab_size, embed_dim], device).await.unwrap();
        
        let indices: Vec<u32> = (0..seq_len).map(|i| (i * 10) % vocab_size as u32).collect();
        let result = embeddings.embedding(indices.clone()).unwrap();
        assert_eq!(result.shape(), &[seq_len as usize, embed_dim]);
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, vocab_size, embed_dim);
        
        for (r, e) in output.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_embedding_precision() {
        let device = get_test_device().await;

        // Test FP32 precision with typical transformer values
        let embed_data = vec![
            0.123, -0.456, 0.789, -0.234,
            0.567, -0.890, 0.345, -0.678,
            0.901, -0.123, 0.456, -0.789,
        ];
        
        let embeddings = Tensor::from_vec_on(embed_data.clone(), vec![3, 4], device).await.unwrap();
        let indices = vec![2, 1, 0, 1, 2];
        let result = embeddings.embedding(indices.clone()).unwrap();
        
        let output = result.to_vec().unwrap();
        let expected = embedding_cpu(&embed_data, &indices, 3, 4);
        
        // Verify FP32 precision (embeddings are direct lookups, should be exact)
        let max_error = output.iter().zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);
        
        assert!(max_error < 1e-6, "Max error: {} exceeds threshold", max_error);
    }
}
