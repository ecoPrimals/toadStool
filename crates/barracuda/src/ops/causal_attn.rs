//! Causal Attention - GPU-accelerated with causal masking
//!
//! **Deep Debt Principles**:
//! - ✅ Composition over duplication (reuses attention shaders)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for GPT-style models)
//!
//! ## Algorithm
//!
//! ```text
//! Causal mask: position i can only attend to positions 0..=i
//! mask[i,j] = -inf if j > i, else 0
//! attention = softmax((QK^T / sqrt(d_k)) + mask) * V
//! ```
//!
//! **Implementation**: 3-pass GPU execution (reuses 2 attention shaders!)
//! 1. Pass 1: Compute QK^T scores (reuse attention_matmul.wgsl ✅)
//! 2. Pass 2: Apply softmax with causal mask (NEW: causal_attention_softmax.wgsl)
//! 3. Pass 3: Apply weights to values (reuse attention_apply.wgsl ✅)
//!
//! **Deep Debt**: Maximum code reuse - only 1 new shader for masking!
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let q = Tensor::randn(vec![2, 8, 128, 64]).await?;  // [batch, heads, seq, dim]
//! let k = Tensor::randn(vec![2, 8, 128, 64]).await?;
//! let v = Tensor::randn(vec![2, 8, 128, 64]).await?;
//!
//! let output = q.causal_attention(&k, &v)?;  // GPT-style autoregressive attention
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Attention parameters for WGSL shaders (same as regular attention)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
}

/// Causal attention operation
///
/// **Deep Debt**: Composes validated attention shaders + causal mask shader
pub struct CausalAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
}

impl CausalAttention {
    /// Create new causal attention operation
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Result<Self> {
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

        Ok(Self { query, key, value })
    }

    /// Pass 1 shader: Compute QK^T scores (REUSED from attention ✅)
    fn shader_matmul() -> &'static str {
        include_str!("../shaders/attention_matmul.wgsl")
    }

    /// Pass 2 shader: Apply softmax with causal mask (NEW - only shader needed!)
    fn shader_causal_softmax() -> &'static str {
        include_str!("../shaders/causal_attention_softmax.wgsl")
    }

    /// Pass 3 shader: Apply weights to values (REUSED from attention ✅)
    fn shader_apply() -> &'static str {
        include_str!("../shaders/attention_apply.wgsl")
    }

    /// Execute causal attention (3 GPU passes)
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
        let params = AttentionParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            seq_len: seq_len as u32,
            head_dim: head_dim as u32,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Causal Attention Params"),
            size: std::mem::size_of::<AttentionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

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
        
        let shader_matmul = device.compile_shader(Self::shader_matmul(), Some("CausalAttentionMatmul"));
        
        let bgl_matmul = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Causal Attention Matmul BGL"),
            entries: &[
                // Query
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
                // Key
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
                // Scores (output)
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
                // Params
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
            label: Some("Causal Attention Matmul BG"),
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

        let pipeline_layout_matmul = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Causal Attention Matmul Pipeline Layout"),
            bind_group_layouts: &[&bgl_matmul],
            push_constant_ranges: &[],
        });

        let pipeline_matmul = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Causal Attention Matmul Pipeline"),
            layout: Some(&pipeline_layout_matmul),
            module: &shader_matmul,
            entry_point: "main",
        });

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax with causal mask (NEW shader!)
        // ═══════════════════════════════════════════════════════════

        let shader_softmax = device.compile_shader(Self::shader_causal_softmax(), Some("CausalAttentionSoftmax"));

        let bgl_softmax = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Causal Attention Softmax BGL"),
            entries: &[
                // Scores (input)
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
                // Weights (output)
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
                // Params
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
            label: Some("Causal Attention Softmax BG"),
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

        let pipeline_layout_softmax = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Causal Attention Softmax Pipeline Layout"),
            bind_group_layouts: &[&bgl_softmax],
            push_constant_ranges: &[],
        });

        let pipeline_softmax = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Causal Attention Softmax Pipeline"),
            layout: Some(&pipeline_layout_softmax),
            module: &shader_softmax,
            entry_point: "main",
        });

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values (REUSED from attention ✅)
        // ═══════════════════════════════════════════════════════════

        let shader_apply = device.compile_shader(Self::shader_apply(), Some("CausalAttentionApply"));

        let bgl_apply = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Causal Attention Apply BGL"),
            entries: &[
                // Weights (input)
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
                // Value (input)
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
                // Output
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
                // Params
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
            label: Some("Causal Attention Apply BG"),
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

        let pipeline_layout_apply = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Causal Attention Apply Pipeline Layout"),
            bind_group_layouts: &[&bgl_apply],
            push_constant_ranges: &[],
        });

        let pipeline_apply = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Causal Attention Apply Pipeline"),
            layout: Some(&pipeline_layout_apply),
            module: &shader_apply,
            entry_point: "main",
        });

        // ═══════════════════════════════════════════════════════════
        // EXECUTE ALL 3 PASSES
        // ═══════════════════════════════════════════════════════════

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Causal Attention Encoder"),
        });

        // Pass 1: Matmul
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Causal Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);
            let workgroups = ((batch_size * num_heads * seq_len * seq_len) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 2: Causal Softmax
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Causal Attention Softmax Pass"),
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
                label: Some("Causal Attention Apply Pass"),
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
    /// Causal attention (GPT-style autoregressive masking)
    ///
    /// **Deep Debt**: Reuses 2/3 attention shaders + causal mask shader
    ///
    /// # Arguments
    /// - `key`: Key tensor [batch, heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Returns
    /// Output tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Example
    /// ```rust,ignore
    /// let q = Tensor::randn(vec![2, 8, 128, 64]).await?;
    /// let k = Tensor::randn(vec![2, 8, 128, 64]).await?;
    /// let v = Tensor::randn(vec![2, 8, 128, 64]).await?;
    ///
    /// let output = q.causal_attention(&k, &v)?;  // GPT-style
    /// ```
    pub fn causal_attention(self, key: &Self, value: &Self) -> Result<Self> {
        CausalAttention::new(self, key.clone(), value.clone())?.execute()
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
    async fn test_causal_attention_basic() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 2;
        let seq = 8;
        let dim = 16;

        // Create inputs
        let q = Tensor::from_vec_on(vec![0.5; batch * heads * seq * dim], vec![batch, heads, seq, dim], device.clone()).await.unwrap();
        let k = Tensor::from_vec_on(vec![0.5; batch * heads * seq * dim], vec![batch, heads, seq, dim], device.clone()).await.unwrap();
        let v = Tensor::from_vec_on(vec![1.0; batch * heads * seq * dim], vec![batch, heads, seq, dim], device).await.unwrap();

        // Execute
        let output = q.causal_attention(&k, &v).unwrap();

        // Validate shape
        assert_eq!(output.shape(), &[batch, heads, seq, dim]);

        // Validate values are finite
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_causal_attention_single_token() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 1;
        let seq = 1; // Single token - no masking needed
        let dim = 4;

        let q = Tensor::from_vec_on(vec![0.5; batch * heads * seq * dim], vec![batch, heads, seq, dim], device.clone()).await.unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.causal_attention(&k, &v).unwrap();

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_causal_attention_gpt_style() {
        let device = get_test_device().await;

        // GPT-style dimensions
        let batch = 2;
        let heads = 8;
        let seq = 16;
        let dim = 16;

        let q = Tensor::from_vec_on(vec![0.5; batch * heads * seq * dim], vec![batch, heads, seq, dim], device.clone()).await.unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.causal_attention(&k, &v).unwrap();

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }
}
