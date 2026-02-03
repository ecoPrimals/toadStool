//! ALiBi Position Encoding - GPU-accelerated
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (single-pass GPU)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for BLOOM, MPT)
//!
//! ## Algorithm
//!
//! ```text
//! ALiBi adds linear biases to attention scores:
//!
//! slope[h] = 2^(-(8*(h+1) / num_heads))
//! distance[i,j] = |i - j|
//! bias[h,i,j] = -slope[h] * distance[i,j]
//! output[b,h,i,j] = scores[b,h,i,j] + bias[h,i,j]
//! ```
//!
//! **Key Properties**:
//! - No learned parameters (like RoPE)
//! - Linear bias based on distance
//! - Head-specific slopes
//! - Enables "train short, test long" (extrapolates to longer sequences)
//!
//! **Used By**: BLOOM, MPT, CodeGen
//!
//! **Reference**: Press et al., 2021 - "Train Short, Test Long: Attention with Linear Biases Enables Input Length Extrapolation"
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! // Compute attention scores first (before softmax)
//! let scores = query.matmul(&key.transpose())?;  // [batch, heads, seq, seq]
//! 
//! // Apply ALiBi bias
//! let biased_scores = scores.alibi_position()?;
//!
//! // Then apply softmax and attend to values
//! let weights = biased_scores.softmax()?;
//! let output = weights.matmul(&value)?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// ALiBi parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AlibiParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    _padding: u32,
}

/// ALiBi Position Encoding operation
///
/// **Deep Debt**: Single-pass GPU, no learned parameters
pub struct AlibiPosition {
    scores: Tensor,
}

impl AlibiPosition {
    /// Create new ALiBi operation
    ///
    /// **Shape**: [batch, heads, seq_len, seq_len] (attention scores)
    pub fn new(scores: Tensor) -> Result<Self> {
        // Validate shape: must be 4D square attention matrix
        if scores.shape().len() != 4 {
            return Err(BarracudaError::shape_mismatch(
                scores.shape().to_vec(),
                vec![0, 0, 0, 0],
            ));
        }

        // Validate last two dims are equal (square attention matrix)
        let seq_len_1 = scores.shape()[2];
        let seq_len_2 = scores.shape()[3];
        if seq_len_1 != seq_len_2 {
            return Err(BarracudaError::invalid_op(
                "AlibiPosition",
                format!(
                    "Attention matrix must be square, got [{}, {}]",
                    seq_len_1, seq_len_2
                ),
            ));
        }

        Ok(Self { scores })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/alibi_position.wgsl")
    }

    /// Execute ALiBi (single GPU pass)
    ///
    /// **Deep Debt**: Efficient single-pass, no intermediate buffers
    pub fn execute(self) -> Result<Tensor> {
        let device = self.scores.device();
        
        // Extract dimensions
        let shape = self.scores.shape();
        let batch_size = shape[0];
        let num_heads = shape[1];
        let seq_len = shape[2];

        // Create parameters
        let params = AlibiParams {
            batch_size: batch_size as u32,
            num_heads: num_heads as u32,
            seq_len: seq_len as u32,
            _padding: 0,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ALiBi Params"),
            size: std::mem::size_of::<AlibiParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffer (same size as input)
        let output_size = self.scores.len();
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("ALiBi"));

        // Create bind group layout
        let bgl = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ALiBi BGL"),
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
                // Output (biased scores)
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ALiBi BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.scores.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ALiBi Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ALiBi Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ALiBi Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ALiBi Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch: one thread per attention score
            let total = (batch_size * num_heads * seq_len * seq_len) as u32;
            let workgroups = (total + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return output tensor (same shape as input)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, num_heads, seq_len, seq_len],
            device.clone(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Apply ALiBi position encoding to attention scores
    ///
    /// **Deep Debt**: Essential for BLOOM, MPT, CodeGen
    ///
    /// # Arguments
    /// - Input: Attention scores [batch, heads, seq_len, seq_len]
    ///
    /// # Returns
    /// - Biased scores [batch, heads, seq_len, seq_len]
    ///
    /// # Example
    /// ```rust,ignore
    /// // Compute attention scores (before softmax)
    /// let scores = query.matmul(&key.transpose())?;
    ///
    /// // Apply ALiBi bias
    /// let biased = scores.alibi_position()?;  // BLOOM-style
    ///
    /// // Then softmax and apply to values
    /// let weights = biased.softmax()?;
    /// let output = weights.matmul(&value)?;
    /// ```
    pub fn alibi_position(self) -> Result<Self> {
        AlibiPosition::new(self)?.execute()
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
    async fn test_alibi_basic() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 2;
        let seq = 4;

        let scores = Tensor::from_vec_on(
            vec![1.0; batch * heads * seq * seq],
            vec![batch, heads, seq, seq],
            device,
        )
        .await
        .unwrap();

        let output = scores.alibi_position().unwrap();

        assert_eq!(output.shape(), &[batch, heads, seq, seq]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_alibi_single_token() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 1;
        let seq = 1;

        let scores = Tensor::from_vec_on(vec![5.0], vec![batch, heads, seq, seq], device)
            .await
            .unwrap();

        let output = scores.alibi_position().unwrap();

        // Distance=0, no bias added
        let data = output.to_vec().unwrap();
        assert!((data[0] - 5.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_alibi_bloom_dims() {
        let device = get_test_device().await;

        // BLOOM-style dimensions
        let batch = 2;
        let heads = 8;
        let seq = 16;

        let scores = Tensor::from_vec_on(
            vec![0.0; batch * heads * seq * seq],
            vec![batch, heads, seq, seq],
            device,
        )
        .await
        .unwrap();

        let output = scores.alibi_position().unwrap();

        assert_eq!(output.shape(), &[batch, heads, seq, seq]);
        let data = output.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
        // Non-zero (bias should be applied)
        assert!(data.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_alibi_diagonal_zero() {
        let device = get_test_device().await;

        let batch = 1;
        let heads = 1;
        let seq = 4;

        let scores = Tensor::from_vec_on(
            vec![1.0; batch * heads * seq * seq],
            vec![batch, heads, seq, seq],
            device,
        )
        .await
        .unwrap();

        let output = scores.alibi_position().unwrap();
        let data = output.to_vec().unwrap();

        // Diagonal elements (distance=0) should have no bias
        for i in 0..seq {
            let idx = i * seq + i;
            assert!((data[idx] - 1.0).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_alibi_shape_validation() {
        let device = get_test_device().await;

        // Valid: square attention matrix
        let scores = Tensor::from_vec_on(vec![1.0; 1 * 2 * 4 * 4], vec![1, 2, 4, 4], device.clone())
            .await
            .unwrap();
        assert!(scores.alibi_position().is_ok());

        // Invalid: non-square matrix
        let scores = Tensor::from_vec_on(vec![1.0; 1 * 2 * 4 * 8], vec![1, 2, 4, 8], device)
            .await
            .unwrap();
        assert!(scores.alibi_position().is_err());
    }
}
