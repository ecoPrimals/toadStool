//! Multi-Head Attention - Complete attention layer with projections
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
//! MultiHead(Q, K, V) = Concat(head_1, ..., head_h) * W^O
//! where head_i = Attention(Q*W^Q_i, K*W^K_i, V*W^V_i)
//! ```
//!
//! This is the complete attention mechanism used in transformers,
//! including all projection matrices.
//!
//! ## Multi-Pass Execution
//!
//! 1. **Pass 1-3**: Project Q, K, V through weight matrices (`mha_projection.wgsl`)
//! 2. **Pass 4**: Apply scaled dot-product attention
//! 3. **Pass 5**: Project concatenated heads through output matrix (`mha_output.wgsl`)

use crate::error::Result;
use crate::ops::scaled_dot_product_attention::ScaledDotProductAttention;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// MHA projection parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MHAProjectionParams {
    pub batch_size: u32,
    pub seq_len: u32,
    pub d_model: u32,
    pub num_heads: u32,
    pub head_dim: u32,
}

/// Multi-head attention operation
pub struct MultiHeadAttention {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    w_q: Tensor,
    w_k: Tensor,
    w_v: Tensor,
    w_o: Tensor,
    batch_size: usize,
    seq_len: usize,
    d_model: usize,
    num_heads: usize,
    head_dim: usize,
}

impl MultiHeadAttention {
    /// Create a new multi-head attention operation
    ///
    /// # Arguments
    /// - `query`: Query tensor [batch, seq_len, d_model]
    /// - `key`: Key tensor [batch, seq_len, d_model]
    /// - `value`: Value tensor [batch, seq_len, d_model]
    /// - `w_q`: Query projection weights [d_model, d_model]
    /// - `w_k`: Key projection weights [d_model, d_model]
    /// - `w_v`: Value projection weights [d_model, d_model]
    /// - `w_o`: Output projection weights [d_model, d_model]
    /// - `num_heads`: Number of attention heads
    ///
    /// # Returns
    /// Result containing the operation struct, or error if shapes are invalid
    pub fn new(
        query: Tensor,
        key: Tensor,
        value: Tensor,
        w_q: Tensor,
        w_k: Tensor,
        w_v: Tensor,
        w_o: Tensor,
        num_heads: usize,
    ) -> Result<Self> {
        // Validate input shapes
        let q_shape = query.shape();
        let k_shape = key.shape();
        let v_shape = value.shape();

        if q_shape.len() != 3 || k_shape.len() != 3 || v_shape.len() != 3 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "All inputs must be 3D tensors [batch, seq_len, d_model]".to_string(),
            });
        }

        let batch_size = q_shape[0];
        let seq_len = q_shape[1];
        let d_model = q_shape[2];

        // Validate all input shapes match
        if k_shape != q_shape || v_shape != q_shape {
            return Err(crate::error::BarracudaError::shape_mismatch(
                q_shape.to_vec(),
                if k_shape != q_shape { k_shape.to_vec() } else { v_shape.to_vec() },
            ));
        }

        // Validate d_model is divisible by num_heads
        if d_model % num_heads != 0 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!("d_model ({}) must be divisible by num_heads ({})", d_model, num_heads),
            });
        }

        let head_dim = d_model / num_heads;

        // Validate weight shapes
        let w_q_shape = w_q.shape();
        let w_k_shape = w_k.shape();
        let w_v_shape = w_v.shape();
        let w_o_shape = w_o.shape();

        let expected_weight_shape = vec![d_model, d_model];
        if w_q_shape != expected_weight_shape
            || w_k_shape != expected_weight_shape
            || w_v_shape != expected_weight_shape
            || w_o_shape != expected_weight_shape
        {
            return Err(crate::error::BarracudaError::shape_mismatch(
                expected_weight_shape,
                w_q_shape.to_vec(),
            ));
        }

        // Validate devices match
        use std::sync::Arc;
        if !Arc::ptr_eq(query.device(), key.device())
            || !Arc::ptr_eq(query.device(), value.device())
            || !Arc::ptr_eq(query.device(), w_q.device())
            || !Arc::ptr_eq(query.device(), w_k.device())
            || !Arc::ptr_eq(query.device(), w_v.device())
            || !Arc::ptr_eq(query.device(), w_o.device())
        {
            return Err(crate::error::BarracudaError::device(
                "All tensors must be on the same device",
            ));
        }

        Ok(Self {
            query,
            key,
            value,
            w_q,
            w_k,
            w_v,
            w_o,
            batch_size,
            seq_len,
            d_model,
            num_heads,
            head_dim,
        })
    }

    /// Get WGSL shader for MHA projection
    fn wgsl_shader_projection() -> &'static str {
        include_str!("../shaders/mha_projection.wgsl")
    }

    /// Get WGSL shader for MHA output projection
    fn wgsl_shader_output() -> &'static str {
        include_str!("../shaders/mha_output.wgsl")
    }

    /// Execute projection pass
    fn execute_projection(
        &self,
        device: &crate::device::WgpuDevice,
        input: &Tensor,
        weight: &Tensor,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::Buffer> {
        let output_size = self.batch_size * self.num_heads * self.seq_len * self.head_dim;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = MHAProjectionParams {
            batch_size: self.batch_size as u32,
            seq_len: self.seq_len as u32,
            d_model: self.d_model as u32,
            num_heads: self.num_heads as u32,
            head_dim: self.head_dim as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MHA Projection Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader_module = device.compile_shader(
            Self::wgsl_shader_projection(),
            Some("MHA Projection Shader"),
        );

        let bind_group_layout = device.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("MHA Projection Bind Group Layout"),
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
            label: Some("MHA Projection Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight.buffer().as_entire_binding(),
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
                label: Some("MHA Projection Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            },
        );

        let pipeline = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("MHA Projection Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            },
        );

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("MHA Projection Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Dispatch: workgroup_size(16, 16, 1) from shader
        // Each workgroup handles one (batch, head, seq) combination
        let workgroups_x = ((self.batch_size + 15) / 16) as u32;
        let workgroups_y = ((self.num_heads + 15) / 16) as u32;
        let workgroups_z = ((self.seq_len + 15) / 16) as u32;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);

        Ok(output_buffer)
    }

    /// Execute output projection pass
    fn execute_output_projection(
        &self,
        device: &crate::device::WgpuDevice,
        input: &wgpu::Buffer,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::Buffer> {
        let output_size = self.batch_size * self.seq_len * self.d_model;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = MHAProjectionParams {
            batch_size: self.batch_size as u32,
            seq_len: self.seq_len as u32,
            d_model: self.d_model as u32,
            num_heads: self.num_heads as u32,
            head_dim: self.head_dim as u32,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MHA Output Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader_module = device.compile_shader(
            Self::wgsl_shader_output(),
            Some("MHA Output Shader"),
        );

        let bind_group_layout = device.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("MHA Output Bind Group Layout"),
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
            label: Some("MHA Output Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.w_o.buffer().as_entire_binding(),
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
                label: Some("MHA Output Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            },
        );

        let pipeline = device.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("MHA Output Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "main",
            },
        );

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("MHA Output Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Dispatch: workgroup_size(16, 16, 1) from shader
        // Each workgroup handles one (batch, seq, out_dim) combination
        let workgroups_x = ((self.batch_size + 15) / 16) as u32;
        let workgroups_y = ((self.seq_len + 15) / 16) as u32;
        let workgroups_z = ((self.d_model + 15) / 16) as u32;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);

        Ok(output_buffer)
    }

    /// Execute the multi-head attention operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.query.device();

        // Create command encoder for all passes
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MultiHeadAttention Encoder"),
        });

        // ═══════════════════════════════════════════════════════════════
        // PASS 1-3: Project Q, K, V through weight matrices
        // ═══════════════════════════════════════════════════════════════
        let q_proj_buffer = self.execute_projection(&device, &self.query, &self.w_q, &mut encoder)?;
        let k_proj_buffer = self.execute_projection(&device, &self.key, &self.w_k, &mut encoder)?;
        let v_proj_buffer = self.execute_projection(&device, &self.value, &self.w_v, &mut encoder)?;

        // Submit projection passes
        device.queue.submit(Some(encoder.finish()));

        // ═══════════════════════════════════════════════════════════════
        // PASS 4: Apply scaled dot-product attention
        // ═══════════════════════════════════════════════════════════════
        // Create tensors from buffers for attention
        let q_proj = Tensor::from_buffer(
            q_proj_buffer,
            vec![self.batch_size, self.num_heads, self.seq_len, self.head_dim],
            device.clone(),
        );
        let k_proj = Tensor::from_buffer(
            k_proj_buffer,
            vec![self.batch_size, self.num_heads, self.seq_len, self.head_dim],
            device.clone(),
        );
        let v_proj = Tensor::from_buffer(
            v_proj_buffer,
            vec![self.batch_size, self.num_heads, self.seq_len, self.head_dim],
            device.clone(),
        );

        // Apply attention
        let attention_output = ScaledDotProductAttention::new(q_proj, k_proj, v_proj)?.execute()?;

        // ═══════════════════════════════════════════════════════════════
        // PASS 5: Project concatenated heads through output matrix
        // ═══════════════════════════════════════════════════════════════
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MultiHeadAttention Output Encoder"),
        });

        let output_buffer = self.execute_output_projection(&device, attention_output.buffer(), &mut encoder)?;

        // Submit output projection pass
        device.queue.submit(Some(encoder.finish()));

        // Return output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![self.batch_size, self.seq_len, self.d_model],
            device.clone(),
        ))
    }
}

// Note: Tensor::multi_head_attention is implemented in mha.rs

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
    async fn test_multi_head_attention_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        let batch = 1;
        let seq_len = 4;
        let d_model = 8;
        let num_heads = 2;

        let query = create_test_tensor(dev.clone(), vec![batch, seq_len, d_model], 0.5).await.unwrap();
        let key = create_test_tensor(dev.clone(), vec![batch, seq_len, d_model], 0.5).await.unwrap();
        let value = create_test_tensor(dev.clone(), vec![batch, seq_len, d_model], 0.5).await.unwrap();

        let weight_size = d_model * d_model;
        let w_q = create_test_tensor(dev.clone(), vec![d_model, d_model], 0.01).await.unwrap();
        let w_k = create_test_tensor(dev.clone(), vec![d_model, d_model], 0.01).await.unwrap();
        let w_v = create_test_tensor(dev.clone(), vec![d_model, d_model], 0.01).await.unwrap();
        let w_o = create_test_tensor(dev.clone(), vec![d_model, d_model], 0.01).await.unwrap();

        let output = query
            .multi_head_attention(key, value, w_q, w_k, w_v, w_o, num_heads)
            .unwrap();

        assert_eq!(output.shape(), &[batch, seq_len, d_model]);
    }
}
