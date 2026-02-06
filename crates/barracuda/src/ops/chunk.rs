//! Chunk - Split tensor into chunks along dimension
//!
//! **Deep Debt Principles**:
//! - Complete implementation: Uses existing chunk.wgsl shader
//! - Zero hardcoding: All parameters configurable
//! - Self-knowledge: Validates chunk count and dimension
//! - Modern idiomatic Rust: Result<T, E>

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkParams {
    chunk_idx: u32,
    chunk_size: u32,
    split_dim: u32,
    dim_size: u32,
    inner_size: u32,
    outer_size: u32,
    output_size: u32,
    _pad1: u32,
}

pub struct Chunk {
    input: Tensor,
    chunks: usize,
    dim: usize,
}

impl Chunk {
    pub fn new(input: Tensor, chunks: usize, dim: usize) -> Result<Self> {
        if chunks == 0 {
            return Err(BarracudaError::invalid_op(
                "chunk",
                "Cannot split into 0 chunks",
            ));
        }

        let shape = input.shape();
        if dim >= shape.len() {
            return Err(BarracudaError::invalid_op(
                "chunk",
                format!("dim {} exceeds tensor rank {}", dim, shape.len()),
            ));
        }

        let dim_size = shape[dim];
        if dim_size % chunks != 0 {
            return Err(BarracudaError::invalid_op(
                "chunk",
                format!("dimension size {} must be divisible by chunks {}", dim_size, chunks),
            ));
        }

        Ok(Self {
            input,
            chunks,
            dim,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/chunk.wgsl")
    }

    pub fn execute(self) -> Result<Vec<Tensor>> {
        let device = self.input.device();
        let shape = self.input.shape();
        let dim_size = shape[self.dim];
        let chunk_size = dim_size / self.chunks;

        // Compute sizes
        let outer_size: usize = shape[..self.dim].iter().product();
        let inner_size: usize = shape[self.dim + 1..].iter().product();
        let output_size = outer_size * chunk_size * inner_size;

        let mut results = Vec::with_capacity(self.chunks);

        for chunk_idx in 0..self.chunks {
            let params = ChunkParams {
                chunk_idx: chunk_idx as u32,
                chunk_size: chunk_size as u32,
                split_dim: self.dim as u32,
                dim_size: dim_size as u32,
                inner_size: inner_size as u32,
                outer_size: outer_size as u32,
                output_size: output_size as u32,
                _pad1: 0,
            };

            let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let output_buffer = device.create_buffer_f32(output_size)?;

            let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Chunk Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
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
                ],
            });

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Chunk Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.input.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output_buffer.as_entire_binding(),
                    },
                ],
            });

            let shader = device.compile_shader(Self::wgsl_shader(), Some("Chunk"));
            let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Chunk Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Chunk Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Chunk Encoder"),
            });

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Chunk Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                // Deep Debt Evolution: Capability-based dispatch
                let caps = DeviceCapabilities::from_device(&device);
                let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
                let workgroups = (output_size as u32 + optimal_wg_size - 1) / optimal_wg_size;
                pass.dispatch_workgroups(workgroups, 1, 1);
            }

            device.queue.submit(Some(encoder.finish()));

            // Compute output shape
            let mut output_shape = shape.to_vec();
            output_shape[self.dim] = chunk_size;

            results.push(Tensor::from_buffer(
                output_buffer,
                output_shape,
                device.clone(),
            ));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_chunk_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(
            (0..12).map(|i| i as f32).collect(),
            vec![3, 4],
            device.clone(),
        )
        .await
        .unwrap();
        
        let chunks = Chunk::new(input, 2, 0).unwrap().execute().unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].shape(), &[1, 4]);
        assert_eq!(chunks[1].shape(), &[1, 4]);
    }

    #[tokio::test]
    async fn test_chunk_along_dim() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(
            (0..12).map(|i| i as f32).collect(),
            vec![2, 6],
            device.clone(),
        )
        .await
        .unwrap();
        
        let chunks = Chunk::new(input, 3, 1).unwrap().execute().unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].shape(), &[2, 2]);
    }

    #[tokio::test]
    async fn test_chunk_invalid() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        
        assert!(Chunk::new(input.clone(), 0, 0).is_err());
        assert!(Chunk::new(input.clone(), 2, 0).is_err()); // 3 not divisible by 2
    }
}
