//! Scaled Dot-Product Attention - Transformer core operation
//!
//! ## Deep Debt Principles
//!
//! - **Pure GPU**: Multi-pass WGSL execution (no CPU fallbacks)
//! - **Zero hardcoding**: Runtime shape validation
//! - **Production-ready**: Complete implementation with proper error handling
//! - **Hardware-agnostic**: Pure WGSL for universal compute
//!
//! ## Algorithm
//!
//! ```text
//! Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) * V
//! ```
//!
//! Where:
//! - Q: Query matrix [batch, heads, seq_len, head_dim]
//! - K: Key matrix [batch, heads, seq_len, head_dim]
//! - V: Value matrix [batch, heads, seq_len, head_dim]
//! - d_k: Dimension of keys (head_dim)
//!
//! ## Multi-Pass Execution
//!
//! 1. **Pass 1**: Compute Q @ K^T scores (`attention_matmul.wgsl`)
//! 2. **Pass 2**: Apply softmax to scores (`attention_softmax.wgsl`)
//! 3. **Pass 3**: Apply attention weights to values (`attention_apply.wgsl`)
//!
//! ## Reference
//!
//! "Attention is All You Need" (Vaswani et al., 2017)
//! https://arxiv.org/abs/1706.03762

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Attention parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AttentionParams {
    pub batch_size: u32,
    pub num_heads: u32,
    pub seq_len: u32,
    pub head_dim: u32,
}

/// Scaled dot-product attention operation
pub struct ScaledDotProductAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    batch_size: usize,
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
}

impl ScaledDotProductAttention {
    /// Create a new scaled dot-product attention operation
    ///
    /// # Arguments
    /// - `query`: Query tensor [batch, heads, seq_len, head_dim]
    /// - `key`: Key tensor [batch, heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Returns
    /// Result containing the operation struct, or error if shapes are invalid
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Result<Self> {
        // Validate shapes match
        let q_shape = query.shape();
        let k_shape = key.shape();
        let v_shape = value.shape();

        if q_shape.len() != 4 || k_shape.len() != 4 || v_shape.len() != 4 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "All inputs must be 4D tensors [batch, heads, seq_len, head_dim]".to_string(),
            });
        }

        let batch_size = q_shape[0];
        let num_heads = q_shape[1];
        let seq_len = q_shape[2];
        let head_dim = q_shape[3];

        // Validate all shapes match
        if k_shape != q_shape || v_shape != q_shape {
            return Err(crate::error::BarracudaError::shape_mismatch(
                q_shape.to_vec(),
                if k_shape != q_shape { k_shape.to_vec() } else { v_shape.to_vec() },
            ));
        }

        // Validate devices match (compare Arc pointers)
        use std::sync::Arc;
        if !Arc::ptr_eq(query.device(), key.device())
            || !Arc::ptr_eq(query.device(), value.device())
        {
            return Err(crate::error::BarracudaError::device(
                "All tensors must be on the same device",
            ));
        }

        Ok(Self {
            query,
            key,
            value,
            batch_size,
            num_heads,
            seq_len,
            head_dim,
        })
    }

    /// Get WGSL shader for attention matrix multiplication (Pass 1)
    fn wgsl_shader_matmul() -> &'static str {
        include_str!("../shaders/attention_matmul.wgsl")
    }

    /// Get WGSL shader for attention softmax (Pass 2)
    fn wgsl_shader_softmax() -> &'static str {
        include_str!("../shaders/attention_softmax.wgsl")
    }

    /// Get WGSL shader for attention apply (Pass 3)
    fn wgsl_shader_apply() -> &'static str {
        include_str!("../shaders/attention_apply.wgsl")
    }

    /// Execute the scaled dot-product attention operation
    ///
    /// Performs multi-pass execution:
    /// 1. Compute Q @ K^T scores
    /// 2. Apply softmax to scores
    /// 3. Apply attention weights to values
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query.device();

        // Calculate buffer sizes
        let input_size = self.batch_size * self.num_heads * self.seq_len * self.head_dim;
        let scores_size = self.batch_size * self.num_heads * self.seq_len * self.seq_len;

        // Create intermediate buffers
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(scores_size)?;
        let output_buffer = device.create_buffer_f32(input_size)?;

        // Create parameters buffer
        let params = AttentionParams {
            batch_size: self.batch_size as u32,
            num_heads: self.num_heads as u32,
            seq_len: self.seq_len as u32,
            head_dim: self.head_dim as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Attention Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create command encoder for all passes
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ScaledDotProductAttention Encoder"),
        });

        // ═══════════════════════════════════════════════════════════════
        // PASS 1: Compute Q @ K^T scores
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_matmul(),
                Some("Attention MatMul Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention MatMul Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Attention MatMul Bind Group"),
                layout: &bind_group_layout,
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

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention MatMul Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Attention MatMul Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention MatMul Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(16, 16, 1) from shader
            // Each workgroup handles one (i, j) pair for one batch-head
            let workgroups_x = ((self.seq_len + 15) / 16) as u32;
            let workgroups_y = ((self.seq_len + 15) / 16) as u32;
            let workgroups_z = (self.batch_size * self.num_heads) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 2: Apply softmax to scores
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_softmax(),
                Some("Attention Softmax Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Softmax Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Attention Softmax Bind Group"),
                layout: &bind_group_layout,
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

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Softmax Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Softmax Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(256) from shader
            // Each thread handles one query position (one row of scores)
            let total_rows = self.batch_size * self.num_heads * self.seq_len;
            compute_pass.dispatch_workgroups((total_rows as u32 + 255) / 256, 1, 1);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 3: Apply attention weights to values
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_apply(),
                Some("Attention Apply Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("Attention Apply Bind Group Layout"),
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
                },
            );

            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Attention Apply Bind Group"),
                layout: &bind_group_layout,
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

            let pipeline_layout = device.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Attention Apply Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Attention Apply Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Attention Apply Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(16, 16, 1) from shader
            // Each workgroup handles one (i, d) pair for one batch-head
            let workgroups_x = ((self.head_dim + 15) / 16) as u32;
            let workgroups_y = ((self.seq_len + 15) / 16) as u32;
            let workgroups_z = (self.batch_size * self.num_heads) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // Submit all passes
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![self.batch_size, self.num_heads, self.seq_len, self.head_dim],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Scaled dot-product attention
    ///
    /// Computes: Attention(Q, K, V) = softmax(QK^T / sqrt(d_k)) * V
    ///
    /// # Arguments
    /// - `key`: Key tensor [batch, heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, heads, seq_len, head_dim]
    ///
    /// # Returns
    /// Output tensor [batch, heads, seq_len, head_dim]
    pub fn scaled_dot_product_attention(
        self,
        key: Tensor,
        value: Tensor,
    ) -> Result<Self> {
        ScaledDotProductAttention::new(self, key, value)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn create_test_tensor(
        device: Arc<WgpuDevice>,
        shape: Vec<usize>,
        value: f32,
    ) -> Result<Tensor> {
        let size: usize = shape.iter().product();
        let data: Vec<f32> = vec![value; size];
        Tensor::from_vec_on(data, shape, device).await
    }

    #[tokio::test]
    async fn test_scaled_dot_product_attention_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Small example: 1 batch, 1 head, 2 seq, 2 dim
        let query = create_test_tensor(dev.clone(), vec![1, 1, 2, 2], 0.5).await.unwrap();
        let key = create_test_tensor(dev.clone(), vec![1, 1, 2, 2], 0.5).await.unwrap();
        let value = create_test_tensor(dev.clone(), vec![1, 1, 2, 2], 1.0).await.unwrap();

        let output = query
            .scaled_dot_product_attention(key, value)
            .unwrap();

        assert_eq!(output.shape(), &[1, 1, 2, 2]);
    }

    #[tokio::test]
    async fn test_scaled_dot_product_attention_multi_head() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Multi-head: 2 batch, 4 heads, 8 seq, 16 dim
        let query = create_test_tensor(dev.clone(), vec![2, 4, 8, 16], 0.5).await.unwrap();
        let key = create_test_tensor(dev.clone(), vec![2, 4, 8, 16], 0.5).await.unwrap();
        let value = create_test_tensor(dev.clone(), vec![2, 4, 8, 16], 1.0).await.unwrap();

        let output = query
            .scaled_dot_product_attention(key, value)
            .unwrap();

        assert_eq!(output.shape(), &[2, 4, 8, 16]);
    }

    #[tokio::test]
    async fn test_scaled_dot_product_attention_shape_validation() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        let query = create_test_tensor(dev.clone(), vec![1, 1, 4, 4], 0.5).await.unwrap();
        let key = create_test_tensor(dev.clone(), vec![1, 1, 4, 4], 0.5).await.unwrap();
        let value = create_test_tensor(dev.clone(), vec![1, 1, 4, 5], 1.0).await.unwrap(); // Wrong shape

        let result = query.scaled_dot_product_attention(key, value);
        assert!(result.is_err()); // Should fail shape validation
    }
}
