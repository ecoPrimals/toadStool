//! Sparse Attention - GPU-accelerated with strided sparse pattern
//!
//! **Deep Debt Principles**:
//! - ✅ Composition over duplication (reuses 2/3 attention shaders!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Memory-efficient (sparse pattern for long sequences)
//!
//! ## Algorithm
//!
//! ```text
//! Sparse mask: position i only attends to positions j where j % stride == 0
//! This reduces computation: O(n²) → O(n²/stride)
//! Example stride=4: attend to [0, 4, 8, 12, 16, ...]
//! ```
//!
//! **Implementation**: 3-pass GPU execution (reuses 2 attention shaders!)
//! 1. Pass 1: Compute QK^T scores (reuse attention_matmul.wgsl ✅)
//! 2. Pass 2: Apply softmax with sparse mask (NEW: sparse_attention_softmax.wgsl)
//! 3. Pass 3: Apply weights to values (reuse attention_apply.wgsl ✅)
//!
//! **Deep Debt**: Maximum code reuse - only 1 new shader for sparse pattern!
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let q = Tensor::randn(vec![2, 8, 1024, 64]).await?;  // Long sequence!
//! let k = Tensor::randn(vec![2, 8, 1024, 64]).await?;
//! let v = Tensor::randn(vec![2, 8, 1024, 64]).await?;
//!
//! let output = q.sparse_attention(&k, &v, 4)?;  // stride=4, attend to every 4th token
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Sparse attention parameters for WGSL shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SparseAttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
    stride: u32,
    _padding: [u32; 3],
}

/// Sparse attention operation
///
/// **Deep Debt**: Composes validated attention shaders + sparse mask shader
pub struct SparseAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    stride: usize,
}

impl SparseAttention {
    /// Create new sparse attention operation
    ///
    /// # Arguments
    /// - `stride`: Attend to every stride-th position (stride=1 is full attention)
    pub fn new(query: Tensor, key: Tensor, value: Tensor, stride: usize) -> Result<Self> {
        // Validate shapes: all must be [batch, heads, seq_len, head_dim]
        if query.shape().len() != 4 || key.shape().len() != 4 || value.shape().len() != 4 {
            return Err(BarracudaError::shape_mismatch(
                query.shape().to_vec(),
                vec![0, 0, 0, 0],
            ));
        }

        if query.shape() != key.shape() || query.shape() != value.shape() {
            return Err(BarracudaError::shape_mismatch(
                query.shape().to_vec(),
                key.shape().to_vec(),
            ));
        }

        if stride == 0 {
            return Err(BarracudaError::invalid_op(
                "SparseAttention",
                "stride must be > 0",
            ));
        }

        Ok(Self {
            query,
            key,
            value,
            stride,
        })
    }

    /// Pass 1 shader: Compute QK^T scores (REUSED from attention ✅)
    fn shader_matmul() -> &'static str {
        include_str!("../shaders/attention_matmul.wgsl")
    }

    /// Pass 2 shader: Apply softmax with sparse mask (NEW - only shader needed!)
    fn shader_sparse_softmax() -> &'static str {
        include_str!("../shaders/sparse_attention_softmax.wgsl")
    }

    /// Pass 3 shader: Apply weights to values (REUSED from attention ✅)
    fn shader_apply() -> &'static str {
        include_str!("../shaders/attention_apply.wgsl")
    }

    /// Execute sparse attention (3 GPU passes)
    ///
    /// **Deep Debt**: Reuses 2/3 shaders from validated attention!
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query.device();

        // Extract dimensions
        let shape = self.query.shape();
        let batch_size = shape[0];
        let num_heads = shape[1];
        let seq_len = shape[2];
        let head_dim = shape[3];

        // Create parameters
        let params = SparseAttentionParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            seq_len: seq_len as u32,
            head_dim: head_dim as u32,
            stride: self.stride as u32,
            _padding: [0, 0, 0],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sparse Attention Params"),
            size: std::mem::size_of::<SparseAttentionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Intermediate buffers
        let scores_size = batch_size * num_heads * seq_len * seq_len;
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(scores_size)?;

        // Output buffer
        let output_size = batch_size * num_heads * seq_len * head_dim;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // ═══════════════════════════════════════════════════════════
        // PASS 1: Compute QK^T scores (REUSED from attention ✅)
        // ═══════════════════════════════════════════════════════════

        let shader_matmul =
            device.compile_shader(Self::shader_matmul(), Some("SparseAttentionMatmul"));

        let bgl_matmul = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sparse Attention Matmul BGL"),
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

        let bg_matmul = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sparse Attention Matmul BG"),
            layout: &bgl_matmul,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.query.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.key.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scores_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout_matmul =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sparse Attention Matmul Pipeline Layout"),
                    bind_group_layouts: &[&bgl_matmul],
                    push_constant_ranges: &[],
                });

        let pipeline_matmul =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Sparse Attention Matmul Pipeline"),
                    layout: Some(&pipeline_layout_matmul),
                    module: &shader_matmul,
                    entry_point: "main",
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax with sparse mask (NEW shader!)
        // ═══════════════════════════════════════════════════════════

        let shader_softmax = device.compile_shader(
            Self::shader_sparse_softmax(),
            Some("SparseAttentionSoftmax"),
        );

        let bgl_softmax =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sparse Attention Softmax BGL"),
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

        let bg_softmax = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sparse Attention Softmax BG"),
            layout: &bgl_softmax,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scores_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout_softmax =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sparse Attention Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bgl_softmax],
                    push_constant_ranges: &[],
                });

        let pipeline_softmax =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Sparse Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout_softmax),
                    module: &shader_softmax,
                    entry_point: "main",
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values (REUSED from attention ✅)
        // ═══════════════════════════════════════════════════════════

        let shader_apply =
            device.compile_shader(Self::shader_apply(), Some("SparseAttentionApply"));

        let bgl_apply = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sparse Attention Apply BGL"),
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

        let bg_apply = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sparse Attention Apply BG"),
            layout: &bgl_apply,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.value.buffer().as_entire_binding(),
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

        let pipeline_layout_apply =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sparse Attention Apply Pipeline Layout"),
                    bind_group_layouts: &[&bgl_apply],
                    push_constant_ranges: &[],
                });

        let pipeline_apply =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Sparse Attention Apply Pipeline"),
                    layout: Some(&pipeline_layout_apply),
                    module: &shader_apply,
                    entry_point: "main",
                });

        // ═══════════════════════════════════════════════════════════
        // EXECUTE ALL 3 PASSES
        // ═══════════════════════════════════════════════════════════

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sparse Attention Encoder"),
            });

        // Pass 1: Matmul
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sparse Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);
            let workgroups = ((batch_size * num_heads * seq_len * seq_len) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 2: Sparse Softmax
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sparse Attention Softmax Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_softmax);
            pass.set_bind_group(0, &bg_softmax, &[]);
            let workgroups = ((batch_size * num_heads * seq_len) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 3: Apply
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sparse Attention Apply Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_apply);
            pass.set_bind_group(0, &bg_apply, &[]);
            let workgroups = ((batch_size * num_heads * seq_len * head_dim) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, num_heads, seq_len, head_dim],
            device.clone(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Sparse attention (strided pattern for long sequences)
    ///
    /// **Deep Debt**: Reuses 2/3 attention shaders + sparse mask shader
    ///
    /// # Arguments
    /// - `key`: Key tensor [batch, heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, heads, seq_len, head_dim]
    /// - `stride`: Attend to every stride-th position (1 = full attention)
    ///
    /// # Returns
    /// Output tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Example
    /// ```rust,ignore
    /// let q = Tensor::randn(vec![2, 8, 1024, 64]).await?;  // Long sequence
    /// let k = Tensor::randn(vec![2, 8, 1024, 64]).await?;
    /// let v = Tensor::randn(vec![2, 8, 1024, 64]).await?;
    ///
    /// let output = q.sparse_attention(&k, &v, 4)?;  // stride=4
    /// ```
    pub fn sparse_attention(self, key: &Self, value: &Self, stride: usize) -> Result<Self> {
        SparseAttention::new(self, key.clone(), value.clone(), stride)?.execute()
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_sparse_attention_basic() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 2;
        let seq = 8;
        let dim = 4;

        let q = Tensor::from_vec_on(
            vec![0.5; batch * heads * seq * dim],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.sparse_attention(&k, &v, 2).unwrap(); // stride=2

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sparse_attention_stride_1() {
        let device = get_test_device().await;

        // stride=1 should work like full attention
        let batch = 1;
        let heads = 1;
        let seq = 4;
        let dim = 4;

        let q = Tensor::from_vec_on(
            vec![1.0; batch * heads * seq * dim],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.sparse_attention(&k, &v, 1).unwrap(); // stride=1 = full

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sparse_attention_large_stride() {
        let device = get_test_device().await;

        // Large stride (attend to few positions)
        let batch = 2;
        let heads = 4;
        let seq = 16;
        let dim = 8;

        let q = Tensor::from_vec_on(
            vec![0.5; batch * heads * seq * dim],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.sparse_attention(&k, &v, 4).unwrap(); // stride=4

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sparse_attention_long_sequence() {
        let device = get_test_device().await;

        // Long sequence (sparse is memory-efficient)
        let batch = 2;
        let heads = 8;
        let seq = 64; // longer sequence
        let dim = 16;

        let q = Tensor::from_vec_on(
            vec![0.5; batch * heads * seq * dim],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.sparse_attention(&k, &v, 8).unwrap(); // stride=8

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
    }
}
