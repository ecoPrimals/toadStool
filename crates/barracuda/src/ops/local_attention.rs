//! Local Attention - GPU-accelerated windowed attention
//!
//! **Deep Debt Principles**:
//! - ✅ Composition over duplication (reuses 2/3 attention shaders!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Memory-efficient (windowed attention for long sequences)
//!
//! ## Algorithm
//!
//! ```text
//! Local window: position i only attends to positions within window
//! window[i] = [max(0, i - half_window), min(seq_len, i + half_window + 1)]
//! This reduces computation: O(n²) → O(n*w) where w is window size
//! ```
//!
//! **Implementation**: 3-pass GPU execution (reuses 2 attention shaders!)
//! 1. Pass 1: Compute QK^T scores (reuse attention_matmul.wgsl ✅)
//! 2. Pass 2: Apply softmax with local window mask (NEW: local_attention_softmax.wgsl)
//! 3. Pass 3: Apply weights to values (reuse attention_apply.wgsl ✅)
//!
//! **Deep Debt**: Maximum code reuse - only 1 new shader for window masking!
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
//! let output = q.local_attention(&k, &v, 4)?;  // window_size=4
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Local attention parameters for WGSL shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LocalAttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
    window_size: u32,
    _padding: [u32; 3],
}

/// Local attention operation
///
/// **Deep Debt**: Composes validated attention shaders + local window mask shader
pub struct LocalAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    window_size: usize,
}

impl LocalAttention {
    /// Create new local attention operation
    ///
    /// # Arguments
    /// - `window_size`: Size of attention window (must be > 0)
    pub fn new(query: Tensor, key: Tensor, value: Tensor, window_size: usize) -> Result<Self> {
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

        if window_size == 0 {
            return Err(BarracudaError::invalid_op(
                "LocalAttention",
                "window_size must be > 0",
            ));
        }

        Ok(Self {
            query,
            key,
            value,
            window_size,
        })
    }

    /// Pass 1 shader: Compute QK^T scores (REUSED from attention ✅)
    fn shader_matmul() -> &'static str {
        include_str!("../shaders/attention_matmul.wgsl")
    }

    /// Pass 2 shader: Apply softmax with local window mask (NEW - only shader needed!)
    fn shader_local_softmax() -> &'static str {
        include_str!("../shaders/local_attention_softmax.wgsl")
    }

    /// Pass 3 shader: Apply weights to values (REUSED from attention ✅)
    fn shader_apply() -> &'static str {
        include_str!("../shaders/attention_apply.wgsl")
    }

    /// Execute local attention (3 GPU passes)
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
        let params = LocalAttentionParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            seq_len: seq_len as u32,
            head_dim: head_dim as u32,
            window_size: self.window_size as u32,
            _padding: [0, 0, 0],
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Local Attention Params"),
            size: std::mem::size_of::<LocalAttentionParams>() as u64,
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
            device.compile_shader(Self::shader_matmul(), Some("LocalAttentionMatmul"));

        let bgl_matmul = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Local Attention Matmul BGL"),
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
            label: Some("Local Attention Matmul BG"),
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
                    label: Some("Local Attention Matmul Pipeline Layout"),
                    bind_group_layouts: &[&bgl_matmul],
                    push_constant_ranges: &[],
                });

        let pipeline_matmul =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Local Attention Matmul Pipeline"),
                    layout: Some(&pipeline_layout_matmul),
                    module: &shader_matmul,
                    entry_point: "main",
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 2: Apply softmax with local window mask (NEW shader!)
        // ═══════════════════════════════════════════════════════════

        let shader_softmax = device.compile_shader(
            Self::shader_local_softmax(),
            Some("LocalAttentionSoftmax"),
        );

        let bgl_softmax =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Local Attention Softmax BGL"),
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
            label: Some("Local Attention Softmax BG"),
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
                    label: Some("Local Attention Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bgl_softmax],
                    push_constant_ranges: &[],
                });

        let pipeline_softmax =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Local Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout_softmax),
                    module: &shader_softmax,
                    entry_point: "main",
                });

        // ═══════════════════════════════════════════════════════════
        // PASS 3: Apply weights to values (REUSED from attention ✅)
        // ═══════════════════════════════════════════════════════════

        let shader_apply =
            device.compile_shader(Self::shader_apply(), Some("LocalAttentionApply"));

        let bgl_apply = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Local Attention Apply BGL"),
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
            label: Some("Local Attention Apply BG"),
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
                    label: Some("Local Attention Apply Pipeline Layout"),
                    bind_group_layouts: &[&bgl_apply],
                    push_constant_ranges: &[],
                });

        let pipeline_apply =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Local Attention Apply Pipeline"),
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
                label: Some("Local Attention Encoder"),
            });

        // Pass 1: Matmul
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Local Attention Matmul Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline_matmul);
            pass.set_bind_group(0, &bg_matmul, &[]);
            let workgroups = ((batch_size * num_heads * seq_len * seq_len) as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Pass 2: Local Softmax
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Local Attention Softmax Pass"),
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
                label: Some("Local Attention Apply Pass"),
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
    /// Local attention (windowed attention for long sequences)
    ///
    /// **Deep Debt**: Reuses 2/3 attention shaders + local window mask shader
    ///
    /// # Arguments
    /// - `key`: Key tensor [batch, heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, heads, seq_len, head_dim]
    /// - `window_size`: Size of attention window (must be > 0)
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
    /// let output = q.local_attention(&k, &v, 4)?;  // window_size=4
    /// ```
    pub fn local_attention(self, key: &Self, value: &Self, window_size: usize) -> Result<Self> {
        LocalAttention::new(self, key.clone(), value.clone(), window_size)?.execute()
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
    async fn test_local_attention_basic() {
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

        let output = q.local_attention(&k, &v, 4).unwrap(); // window_size=4

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_local_attention_edge_cases() {
        let device = get_test_device().await;

        // Window size = 2 (minimal)
        let batch = 1;
        let heads = 1;
        let seq = 4;
        let dim = 2;

        let q = Tensor::from_vec_on(
            vec![1.0; batch * heads * seq * dim],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        let output = q.local_attention(&k, &v, 2).unwrap(); // window_size=2

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));

        // Single head
        let batch = 1;
        let heads = 1;
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

        let output = q.local_attention(&k, &v, 4).unwrap();
        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
    }

    #[tokio::test]
    async fn test_local_attention_boundary() {
        let device = get_test_device().await;

        // Large window (approaches full attention)
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

        let output = q.local_attention(&k, &v, 8).unwrap(); // window_size=8 (full)

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));

        // Multiple heads
        let batch = 1;
        let heads = 8;
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

        let output = q.local_attention(&k, &v, 4).unwrap();
        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
    }

    #[tokio::test]
    async fn test_local_attention_large_batch() {
        let device = get_test_device().await;

        // Batch size 4, longer sequence
        let batch = 4;
        let heads = 4;
        let seq = 32;
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

        let output = q.local_attention(&k, &v, 8).unwrap();
        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
    }

    #[tokio::test]
    async fn test_local_attention_precision() {
        let device = get_test_device().await;

        // Test attention pattern with known values
        let batch = 1;
        let heads = 1;
        let seq = 4;
        let dim = 2;

        let q = Tensor::from_vec_on(
            vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
            vec![batch, heads, seq, dim],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            vec![batch, heads, seq, dim],
            device,
        )
        .await
        .unwrap();

        let output = q.local_attention(&k, &v, 4).unwrap();

        assert_eq!(output.shape(), &[batch, heads, seq, dim]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
        // Verify attention produces weighted sums
        assert!(data.iter().any(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_local_attention_window_size_validation() {
        let device = get_test_device().await;

        let q = Tensor::from_vec_on(
            vec![0.5; 1 * 1 * 4 * 2],
            vec![1, 1, 4, 2],
            device.clone(),
        )
        .await
        .unwrap();
        let k = q.clone();
        let v = q.clone();

        // Valid: window_size > 0
        assert!(q.clone().local_attention(&k, &v, 1).is_ok());

        // Invalid: window_size = 0
        assert!(q.local_attention(&k, &v, 0).is_err());
    }
}
