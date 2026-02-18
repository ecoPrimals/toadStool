//! Cosine Similarity (f64) — GPU-Accelerated via WGSL
//!
//! Computes cosine similarity: sim(a,b) = (a·b) / (||a|| * ||b||)
//!
//! **Use cases**:
//! - MS2 spectral matching in analytical chemistry (wetSpring)
//! - High-precision similarity search
//! - Biological sequence comparison
//!
//! **Note**: For large batches, `GemmF64 + FusedMapReduceF64` is often faster.
//! This dedicated shader is optimal for single-pair or small-batch queries
//! where GEMM overhead dominates.
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (hardware-agnostic)
//! - Full f64 precision for science-grade accuracy
//! - Safe Rust wrapper (no unsafe code)

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for cosine similarity shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CosineParams {
    num_vectors_a: u32,
    num_vectors_b: u32,
    vector_dim: u32,
    _padding: u32,
}

/// GPU-accelerated f64 cosine similarity
pub struct CosineSimilarityF64 {
    device: Arc<WgpuDevice>,
}

impl CosineSimilarityF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/math/cosine_similarity_f64.wgsl")
    }

    /// Create a new CosineSimilarityF64 orchestrator
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    /// Compute cosine similarity between two single vectors
    ///
    /// # Arguments
    /// * `a` - First vector (f64)
    /// * `b` - Second vector (f64)
    ///
    /// # Returns
    /// Cosine similarity in [-1, 1]
    pub fn similarity(&self, a: &[f64], b: &[f64]) -> Result<f64> {
        if a.len() != b.len() {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Vector dimensions must match: a={}, b={}",
                    a.len(),
                    b.len()
                ),
            });
        }

        let n = a.len();
        if n == 0 {
            return Err(BarracudaError::InvalidInput {
                message: "Empty vectors".to_string(),
            });
        }

        // CPU for small vectors
        if n < 256 {
            return Ok(self.similarity_cpu(a, b));
        }

        let matrix = self.all_pairs_gpu(&[a.to_vec()], &[b.to_vec()], n)?;
        Ok(matrix[0])
    }

    /// Compute all-pairs cosine similarity matrix
    ///
    /// # Arguments
    /// * `vectors_a` - First set of vectors (each of dimension `dim`)
    /// * `vectors_b` - Second set of vectors (each of dimension `dim`)
    /// * `dim` - Vector dimension
    ///
    /// # Returns
    /// Similarity matrix of shape [len(vectors_a), len(vectors_b)]
    /// Row-major: result[i * len(vectors_b) + j] = sim(vectors_a[i], vectors_b[j])
    pub fn all_pairs(
        &self,
        vectors_a: &[Vec<f64>],
        vectors_b: &[Vec<f64>],
        dim: usize,
    ) -> Result<Vec<f64>> {
        if vectors_a.is_empty() || vectors_b.is_empty() {
            return Ok(vec![]);
        }

        // Validate dimensions
        for (i, v) in vectors_a.iter().enumerate() {
            if v.len() != dim {
                return Err(BarracudaError::InvalidInput {
                    message: format!("vectors_a[{}] has dim {}, expected {}", i, v.len(), dim),
                });
            }
        }
        for (i, v) in vectors_b.iter().enumerate() {
            if v.len() != dim {
                return Err(BarracudaError::InvalidInput {
                    message: format!("vectors_b[{}] has dim {}, expected {}", i, v.len(), dim),
                });
            }
        }

        // CPU fallback for small inputs
        let total_pairs = vectors_a.len() * vectors_b.len();
        if total_pairs < 64 || dim < 32 {
            return Ok(self.all_pairs_cpu(vectors_a, vectors_b));
        }

        self.all_pairs_gpu(vectors_a, vectors_b, dim)
    }

    /// CPU reference implementation (single pair)
    fn similarity_cpu(&self, a: &[f64], b: &[f64]) -> f64 {
        let mut dot = 0.0f64;
        let mut norm_a = 0.0f64;
        let mut norm_b = 0.0f64;

        for (ai, bi) in a.iter().zip(b.iter()) {
            dot += ai * bi;
            norm_a += ai * ai;
            norm_b += bi * bi;
        }

        let denom = (norm_a * norm_b).sqrt();
        if denom < 1e-14 {
            return 0.0;
        }
        dot / denom
    }

    /// CPU reference implementation (all pairs)
    fn all_pairs_cpu(&self, vectors_a: &[Vec<f64>], vectors_b: &[Vec<f64>]) -> Vec<f64> {
        let mut result = Vec::with_capacity(vectors_a.len() * vectors_b.len());
        for va in vectors_a {
            for vb in vectors_b {
                result.push(self.similarity_cpu(va, vb));
            }
        }
        result
    }

    fn all_pairs_gpu(
        &self,
        vectors_a: &[Vec<f64>],
        vectors_b: &[Vec<f64>],
        dim: usize,
    ) -> Result<Vec<f64>> {
        let num_a = vectors_a.len();
        let num_b = vectors_b.len();

        // Flatten vectors into contiguous arrays
        let a_flat: Vec<f64> = vectors_a.iter().flat_map(|v| v.iter().cloned()).collect();
        let b_flat: Vec<f64> = vectors_b.iter().flat_map(|v| v.iter().cloned()).collect();

        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("Cosine Similarity f64"));

        // Create buffers
        let a_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vectors A"),
                contents: bytemuck::cast_slice(&a_flat),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let b_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vectors B"),
                contents: bytemuck::cast_slice(&b_flat),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = num_a * num_b;
        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: (output_size * 8) as u64, // f64 = 8 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = CosineParams {
            num_vectors_a: num_a as u32,
            num_vectors_b: num_b as u32,
            vector_dim: dim as u32,
            _padding: 0,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cosine BGL"),
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

        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cosine PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cosine Pipeline"),
                layout: Some(&pl),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let bg = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cosine BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        // Dispatch: workgroup size is 16x16
        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cosine Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cosine Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            let wg_x = num_a.div_ceil(16);
            let wg_y = num_b.div_ceil(16);
            pass.dispatch_workgroups(wg_x as u32, wg_y as u32, 1);
        }

        self.device.queue.submit(Some(encoder.finish()));

        // Read back results
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (output_size * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder2 = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder2.copy_buffer_to_buffer(&output_buf, 0, &staging, 0, (output_size * 8) as u64);
        self.device.queue.submit(Some(encoder2.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let results: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    fn get_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_identical_vectors() {
        let Some(device) = get_test_device() else { return; };
        let op = CosineSimilarityF64::new(device).unwrap();

        let a = vec![1.0, 2.0, 3.0, 4.0];
        let sim = op.similarity(&a, &a).unwrap();

        assert!(
            (sim - 1.0).abs() < 1e-10,
            "Expected 1.0 for identical vectors, got {}",
            sim
        );
    }

    #[test]
    fn test_orthogonal_vectors() {
        let Some(device) = get_test_device() else { return; };
        let op = CosineSimilarityF64::new(device).unwrap();

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let sim = op.similarity(&a, &b).unwrap();
        assert!(sim.abs() < 1e-10, "Expected 0 for orthogonal vectors, got {}", sim);
    }

    #[test]
    fn test_opposite_vectors() {
        let Some(device) = get_test_device() else { return; };
        let op = CosineSimilarityF64::new(device).unwrap();

        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];

        let sim = op.similarity(&a, &b).unwrap();
        assert!(
            (sim + 1.0).abs() < 1e-10,
            "Expected -1.0 for opposite vectors, got {}",
            sim
        );
    }

    #[test]
    fn test_all_pairs() {
        let Some(device) = get_test_device() else { return; };
        let op = CosineSimilarityF64::new(device).unwrap();

        let vectors_a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let vectors_b = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];

        let result = op.all_pairs(&vectors_a, &vectors_b, 2).unwrap();

        // Expected: [[1, 0, 0.707], [0, 1, 0.707]]
        assert!((result[0] - 1.0).abs() < 1e-8); // sim([1,0], [1,0]) = 1
        assert!(result[1].abs() < 1e-8); // sim([1,0], [0,1]) = 0
        let sqrt2_inv = 1.0 / 2.0_f64.sqrt();
        assert!((result[2] - sqrt2_inv).abs() < 1e-8); // sim([1,0], [1,1]) = 1/√2
    }

    #[test]
    fn test_large_vectors() {
        let Some(device) = get_test_device() else { return; };
        let op = CosineSimilarityF64::new(device).unwrap();

        let n = 1000;
        let a: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
        let b: Vec<f64> = (0..n).map(|i| (i as f64).cos()).collect();

        let gpu_sim = op.similarity(&a, &b).unwrap();
        let cpu_sim = op.similarity_cpu(&a, &b);

        // GPU and CPU use different reduction orderings, expect ~1e-6 relative error
        let rel_tol = 1e-6;
        let abs_tol = 1e-10;
        let diff = (gpu_sim - cpu_sim).abs();
        let tol = rel_tol * cpu_sim.abs().max(1.0) + abs_tol;
        assert!(
            diff < tol,
            "GPU: {}, CPU: {}, diff: {}, tol: {}",
            gpu_sim,
            cpu_sim,
            diff,
            tol
        );
    }
}
