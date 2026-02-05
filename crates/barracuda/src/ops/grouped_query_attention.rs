//! Grouped Query Attention - LLaMA-style efficient attention
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
//! Multi-Query Attention variant where queries have multiple heads
//! but keys/values share heads across groups.
//!
//! Memory efficient: Reduces KV cache size for inference.
//! Reference: LLaMA, LLaMA-2 (Meta AI)
//!
//! ## Multi-Pass Execution
//!
//! Similar to scaled dot-product attention but with grouped key/value heads:
//! 1. Compute Q @ K^T scores (adapted for grouped queries)
//! 2. Apply softmax to scores
//! 3. Apply attention weights to values

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// GQA attention parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GQAParams {
    pub batch_size: u32,
    pub num_q_heads: u32,
    pub num_kv_heads: u32,
    pub seq_len: u32,
    pub head_dim: u32,
    pub heads_per_group: u32,
}

/// Grouped Query Attention operation
pub struct GroupedQueryAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    batch_size: usize,
    num_q_heads: usize,
    num_kv_heads: usize,
    seq_len: usize,
    head_dim: usize,
    heads_per_group: usize,
}

impl GroupedQueryAttention {
    /// Create a new grouped query attention operation
    ///
    /// # Arguments
    /// - `query`: Query tensor [batch, num_q_heads, seq_len, head_dim]
    /// - `key`: Key tensor [batch, num_kv_heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, num_kv_heads, seq_len, head_dim]
    ///
    /// # Returns
    /// Result containing the operation struct, or error if shapes are invalid
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Result<Self> {
        // Validate shapes
        let q_shape = query.shape();
        let k_shape = key.shape();
        let v_shape = value.shape();

        if q_shape.len() != 4 || k_shape.len() != 4 || v_shape.len() != 4 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "All inputs must be 4D tensors [batch, heads, seq_len, head_dim]".to_string(),
            });
        }

        let batch_size = q_shape[0];
        let num_q_heads = q_shape[1];
        let seq_len = q_shape[2];
        let head_dim = q_shape[3];

        // Validate key/value shapes
        let num_kv_heads = k_shape[1];
        if k_shape[0] != batch_size || k_shape[2] != seq_len || k_shape[3] != head_dim {
            return Err(crate::error::BarracudaError::shape_mismatch(
                vec![batch_size, num_q_heads, seq_len, head_dim],
                k_shape.to_vec(),
            ));
        }

        if v_shape != k_shape {
            return Err(crate::error::BarracudaError::shape_mismatch(
                k_shape.to_vec(),
                v_shape.to_vec(),
            ));
        }

        // Validate num_q_heads is divisible by num_kv_heads
        if num_q_heads % num_kv_heads != 0 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "num_q_heads ({}) must be divisible by num_kv_heads ({})",
                    num_q_heads, num_kv_heads
                ),
            });
        }

        let heads_per_group = num_q_heads / num_kv_heads;

        // Validate devices match
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
            num_q_heads,
            num_kv_heads,
            seq_len,
            head_dim,
            heads_per_group,
        })
    }

    /// Get WGSL shader for GQA attention matrix multiplication (Pass 1)
    fn wgsl_shader_matmul() -> &'static str {
        include_str!("../shaders/gqa_matmul.wgsl")
    }

    /// Get WGSL shader for GQA attention softmax (Pass 2)
    fn wgsl_shader_softmax() -> &'static str {
        include_str!("../shaders/gqa_softmax.wgsl")
    }

    /// Get WGSL shader for GQA attention apply (Pass 3)
    fn wgsl_shader_apply() -> &'static str {
        include_str!("../shaders/gqa_apply.wgsl")
    }

    /// Execute the grouped query attention operation
    ///
    /// Performs multi-pass execution adapted for grouped queries:
    /// 1. Compute Q @ K^T scores (with grouped KV heads)
    /// 2. Apply softmax to scores
    /// 3. Apply attention weights to values
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query.device();

        // Calculate buffer sizes
        // Scores: [batch, num_q_heads, seq_len, seq_len]
        let scores_size = self.batch_size * self.num_q_heads * self.seq_len * self.seq_len;
        let weights_size = scores_size;
        let output_size = self.batch_size * self.num_q_heads * self.seq_len * self.head_dim;

        // Create intermediate buffers
        let scores_buffer = device.create_buffer_f32(scores_size)?;
        let weights_buffer = device.create_buffer_f32(weights_size)?;
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create parameters buffer
        let params = GQAParams {
            batch_size: self.batch_size as u32,
            num_q_heads: self.num_q_heads as u32,
            num_kv_heads: self.num_kv_heads as u32,
            seq_len: self.seq_len as u32,
            head_dim: self.head_dim as u32,
            heads_per_group: self.heads_per_group as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GQA Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create command encoder for all passes
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GroupedQueryAttention Encoder"),
        });

        // ═══════════════════════════════════════════════════════════════
        // PASS 1: Compute Q @ K^T scores (with grouped KV heads)
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_matmul(),
                Some("GQA MatMul Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA MatMul Bind Group Layout"),
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
                label: Some("GQA MatMul Bind Group"),
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
                    label: Some("GQA MatMul Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA MatMul Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA MatMul Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(16, 16, 1) from shader
            // Each workgroup handles one (i, j) pair for one batch-query_head
            let workgroups_x = ((self.seq_len + 15) / 16) as u32;
            let workgroups_y = ((self.seq_len + 15) / 16) as u32;
            let workgroups_z = (self.batch_size * self.num_q_heads) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 2: Apply softmax to scores
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_softmax(),
                Some("GQA Softmax Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA Softmax Bind Group Layout"),
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
                label: Some("GQA Softmax Bind Group"),
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
                    label: Some("GQA Softmax Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA Softmax Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA Softmax Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(256) from shader
            // Each thread handles one query position (one row of scores)
            let total_rows = self.batch_size * self.num_q_heads * self.seq_len;
            compute_pass.dispatch_workgroups((total_rows as u32 + 255) / 256, 1, 1);
        }

        // ═══════════════════════════════════════════════════════════════
        // PASS 3: Apply attention weights to values
        // ═══════════════════════════════════════════════════════════════
        {
            let shader_module = device.compile_shader(
                Self::wgsl_shader_apply(),
                Some("GQA Apply Shader"),
            );

            let bind_group_layout = device.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("GQA Apply Bind Group Layout"),
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
                label: Some("GQA Apply Bind Group"),
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
                    label: Some("GQA Apply Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = device.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("GQA Apply Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                },
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GQA Apply Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch: workgroup_size(16, 16, 1) from shader
            // Each workgroup handles one (i, d) pair for one batch-query_head
            let workgroups_x = ((self.head_dim + 15) / 16) as u32;
            let workgroups_y = ((self.seq_len + 15) / 16) as u32;
            let workgroups_z = (self.batch_size * self.num_q_heads) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        // Submit all passes
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![self.batch_size, self.num_q_heads, self.seq_len, self.head_dim],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Grouped Query Attention
    ///
    /// Computes attention with grouped key/value heads (efficient for inference).
    ///
    /// # Arguments
    /// - `key`: Key tensor [batch, num_kv_heads, seq_len, head_dim]
    /// - `value`: Value tensor [batch, num_kv_heads, seq_len, head_dim]
    ///
    /// # Returns
    /// Output tensor [batch, num_q_heads, seq_len, head_dim]
    pub fn grouped_query_attention(
        self,
        key: Tensor,
        value: Tensor,
    ) -> Result<Self> {
        GroupedQueryAttention::new(self, key, value)?.execute()
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
    async fn test_grouped_query_attention_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // 8 query heads, 2 KV heads (4 heads per group)
        let query = create_test_tensor(dev.clone(), vec![1, 8, 4, 4], 0.5).await.unwrap();
        let key = create_test_tensor(dev.clone(), vec![1, 2, 4, 4], 0.5).await.unwrap();
        let value = create_test_tensor(dev.clone(), vec![1, 2, 4, 4], 0.5).await.unwrap();

        let output = query.grouped_query_attention(key, value).unwrap();

        assert_eq!(output.shape(), &[1, 8, 4, 4]);
    }
}
