//! Scaled Dot-Product Attention - GPU-accelerated implementation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL multi-pass implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! ```text
//! Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) * V
//! ```
//!
//! **Implementation**: 3-pass GPU execution
//! 1. Pass 1: Compute QK^T scores (matrix multiplication)
//! 2. Pass 2: Apply softmax to scores (row-wise)
//! 3. Pass 3: Apply weights to values (weighted sum)
//!
//! **Reference**: "Attention is All You Need" (Vaswani et al., 2017)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let query = Tensor::randn(vec![2, 8, 128, 64]).await?;  // [batch, heads, seq, dim]
//! let key = Tensor::randn(vec![2, 8, 128, 64]).await?;
//! let value = Tensor::randn(vec![2, 8, 128, 64]).await?;
//!
//! let output = query.attention(&key, &value)?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Attention parameters for WGSL shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
}

/// Scaled dot-product attention operation
///
/// **Multi-pass GPU implementation**:
/// - Pass 1: QK^T (attention scores)
/// - Pass 2: Softmax (attention weights)
/// - Pass 3: Apply to V (output)
pub struct Attention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
}

impl Attention {
    /// Create new attention operation
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Result<Self> {
        // Validate shapes: all must be [batch, heads, seq_len, head_dim]
        if query.shape().len() != 4 || key.shape().len() != 4 || value.shape().len() != 4 {
            return Err(BarracudaError::shape_mismatch(
                query.shape().to_vec(),
                vec![0, 0, 0, 0], // Placeholder - we need 4D
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

    /// Pass 1 shader: Compute QK^T scores
    fn shader_matmul() -> &'static str {
        include_str!("../shaders/attention_matmul.wgsl")
    }

    /// Pass 2 shader: Apply softmax
    fn shader_softmax() -> &'static str {
        include_str!("../shaders/attention_softmax.wgsl")
    }

    /// Pass 3 shader: Apply weights to values
    fn shader_apply() -> &'static str {
        include_str!("../shaders/attention_apply.wgsl")
    }

    /// Execute attention operation (3 GPU passes)
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
            label: Some("Attention Params"),
            size: std::mem::size_of::<AttentionParams>() as u64,
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
        // PASS 1: Compute QK^T scores
        // ═══════════════════════════════════════════════════════════

        let shader_matmul = device.compile_shader(Self::shader_matmul(), Some("AttentionMatmul"));

        let bgl_matmul = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Attention Matmul BGL"),
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
            label: Some("Attention Matmul BG"),
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
                    label: Some("Attention Matmul PL"),
                    bind_group_layouts: &[&bgl_matmul],
                    push_constant_ranges: &[],
                });

        let pipeline_matmul =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Matmul Pipeline"),
                    layout: Some(&pipeline_layout_matmul),
                    module: &shader_matmul,
                    entry_point: "main",
                });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Attention Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);

            // Workgroups: [seq_len/16, seq_len/16, batch*heads]
            let workgroups_x = (seq_len as u32 + 15) / 16;
            let workgroups_y = (seq_len as u32 + 15) / 16;
            let workgroups_z = (batch_size * num_heads) as u32;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax
        // ═══════════════════════════════════════════════════════════

        let shader_softmax =
            device.compile_shader(Self::shader_softmax(), Some("AttentionSoftmax"));

        let bgl_softmax =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Softmax BGL"),
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
            label: Some("Attention Softmax BG"),
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
                    label: Some("Attention Softmax PL"),
                    bind_group_layouts: &[&bgl_softmax],
                    push_constant_ranges: &[],
                });

        let pipeline_softmax =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout_softmax),
                    module: &shader_softmax,
                    entry_point: "main",
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Softmax Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_softmax);
            pass.set_bind_group(0, &bg_softmax, &[]);

            // Workgroups: one per [batch, head, query_pos]
            let workgroups = ((batch_size * num_heads * seq_len) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values
        // ═══════════════════════════════════════════════════════════

        let shader_apply = device.compile_shader(Self::shader_apply(), Some("AttentionApply"));

        let bgl_apply = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Attention Apply BGL"),
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
            label: Some("Attention Apply BG"),
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
                    label: Some("Attention Apply PL"),
                    bind_group_layouts: &[&bgl_apply],
                    push_constant_ranges: &[],
                });

        let pipeline_apply =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Apply Pipeline"),
                    layout: Some(&pipeline_layout_apply),
                    module: &shader_apply,
                    entry_point: "main",
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Apply Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_apply);
            pass.set_bind_group(0, &bg_apply, &[]);

            // Workgroups: [head_dim/16, seq_len/16, batch*heads]
            let workgroups_x = (head_dim as u32 + 15) / 16;
            let workgroups_y = (seq_len as u32 + 15) / 16;
            let workgroups_z = (batch_size * num_heads) as u32;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // Submit all passes
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, num_heads, seq_len, head_dim],
            device.clone(),
        ))
    }
}

/// Tensor API integration
impl Tensor {
    /// Scaled dot-product attention
    ///
    /// # Arguments
    ///
    /// * `key` - Key tensor [batch, heads, seq_len, head_dim]
    /// * `value` - Value tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Returns
    ///
    /// Output tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = query.attention(&key, &value)?;
    /// ```
    pub fn attention(self, key: &Self, value: &Self) -> Result<Self> {
        Attention::new(self, key.clone(), value.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_attention_basic() {
        let device = get_test_device().await;

        // Small test: [1 batch, 1 head, 4 seq, 8 dim]
        let query = Tensor::from_vec_on(vec![1.0; 32], vec![1, 1, 4, 8], device.clone())
            .await
            .unwrap();
        let key = Tensor::from_vec_on(vec![1.0; 32], vec![1, 1, 4, 8], device.clone())
            .await
            .unwrap();
        let value = Tensor::from_vec_on(vec![2.0; 32], vec![1, 1, 4, 8], device)
            .await
            .unwrap();

        let output = query.attention(&key, &value).unwrap();

        assert_eq!(output.shape(), &[1, 1, 4, 8]);
        let result = output.to_vec().unwrap();

        // With uniform Q,K, attention weights should be uniform (1/seq_len)
        // So output should be close to value (since all weighted equally)
        assert!(result.iter().all(|&x| (x - 2.0).abs() < 0.1));
    }

    #[tokio::test]
    async fn test_attention_shape_validation() {
        let device = get_test_device().await;

        let query = Tensor::from_vec_on(vec![1.0; 32], vec![1, 1, 4, 8], device.clone())
            .await
            .unwrap();
        let key = Tensor::from_vec_on(vec![1.0; 16], vec![1, 1, 2, 8], device.clone()) // Wrong shape!
            .await
            .unwrap();
        let value = Tensor::from_vec_on(vec![1.0; 32], vec![1, 1, 4, 8], device)
            .await
            .unwrap();

        let result = query.attention(&key, &value);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_attention_multi_head() {
        let device = get_test_device().await;

        // Test with multiple heads: [2 batch, 4 heads, 8 seq, 16 dim]
        let size = 2 * 4 * 8 * 16;
        let query = Tensor::from_vec_on(vec![0.5; size], vec![2, 4, 8, 16], device.clone())
            .await
            .unwrap();
        let key = query.clone();
        let value = Tensor::from_vec_on(vec![1.0; size], vec![2, 4, 8, 16], device)
            .await
            .unwrap();

        let output = query.attention(&key, &value).unwrap();

        assert_eq!(output.shape(), &[2, 4, 8, 16]);
        let result = output.to_vec().unwrap();
        assert_eq!(result.len(), size);
        assert!(result.iter().all(|&x| x.is_finite()));
    }
}
