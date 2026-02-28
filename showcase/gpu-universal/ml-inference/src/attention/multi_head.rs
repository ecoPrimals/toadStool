//! Multi-Head Attention

use anyhow::Result;
use std::sync::Arc;

use super::ScaledDotProductAttention;

/// Multi-Head Attention
///
/// Parallel attention heads with learned linear projections and concatenation.
///
/// ## Architecture
///
/// ```text
/// MultiHead(Q, K, V) = Concat(head_1, ..., head_h) W^O
/// where head_i = Attention(Q W^Q_i, K W^K_i, V W^V_i)
/// ```
///
/// ## Parameters
///
/// - `input`: [batch, seq_len, d_model]
/// - `num_heads`: Number of parallel attention heads
/// - `d_model`: Model dimension (must be divisible by num_heads)
/// - `W_q`, `W_k`, `W_v`: Query/Key/Value projection weights [d_model, d_model]
/// - `W_o`: Output projection weights [d_model, d_model]
///
/// ## Returns
///
/// - Output: [batch, seq_len, d_model]
///
/// ## Performance
///
/// - Complexity: O(num_heads · seq_len² · d_k)
/// - Memory: O(num_heads · seq_len²)
/// - Parallelized across heads
pub struct MultiHeadAttention {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    num_heads: u32,
    d_model: u32,
    d_k: u32, // d_model / num_heads
    d_v: u32, // d_model / num_heads
    attention: ScaledDotProductAttention,
}

impl MultiHeadAttention {
    /// Create new Multi-Head Attention operation
    ///
    /// # Arguments
    ///
    /// * `device` - GPU device
    /// * `queue` - GPU command queue
    /// * `num_heads` - Number of parallel attention heads
    /// * `d_model` - Model dimension (must be divisible by num_heads)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - d_model not divisible by num_heads
    /// - Shader compilation fails
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        num_heads: u32,
        d_model: u32,
    ) -> Result<Self> {
        anyhow::ensure!(
            d_model.is_multiple_of(num_heads),
            "d_model ({d_model}) must be divisible by num_heads ({num_heads})"
        );

        let d_k = d_model / num_heads;
        let d_v = d_model / num_heads;

        // Create underlying scaled dot-product attention
        let attention =
            ScaledDotProductAttention::new(Arc::clone(&device), Arc::clone(&queue)).await?;

        Ok(Self {
            device,
            queue,
            num_heads,
            d_model,
            d_k,
            d_v,
            attention,
        })
    }

    /// Execute multi-head attention
    ///
    /// # Arguments
    ///
    /// * `query` - Query tensor [batch, seq_len, d_model]
    /// * `key` - Key tensor [batch, seq_len, d_model]
    /// * `value` - Value tensor [batch, seq_len, d_model]
    /// * `w_q` - Query projection [d_model, d_model]
    /// * `w_k` - Key projection [d_model, d_model]
    /// * `w_v` - Value projection [d_model, d_model]
    /// * `w_o` - Output projection [d_model, d_model]
    /// * `mask` - Optional attention mask [batch, seq_len, seq_len]
    /// * `batch` - Batch size
    /// * `seq_len` - Sequence length
    ///
    /// # Returns
    ///
    /// Output tensor [batch, seq_len, d_model]
    ///
    /// # Errors
    ///
    /// Returns error if GPU execution fails or dimensions mismatch
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        query: &[f32],
        key: &[f32],
        value: &[f32],
        w_q: &[f32],
        w_k: &[f32],
        w_v: &[f32],
        w_o: &[f32],
        mask: Option<&[f32]>,
        batch: u32,
        seq_len: u32,
    ) -> Result<Vec<f32>> {
        // Validate dimensions
        let expected_input_size = (batch * seq_len * self.d_model) as usize;
        let expected_weight_size = (self.d_model * self.d_model) as usize;

        anyhow::ensure!(
            query.len() == expected_input_size,
            "Query size mismatch: expected {}, got {}",
            expected_input_size,
            query.len()
        );
        anyhow::ensure!(
            w_q.len() == expected_weight_size,
            "W_q size mismatch: expected {}, got {}",
            expected_weight_size,
            w_q.len()
        );

        // Step 1: Linear projections (Q, K, V)
        let q_proj = self.linear_projection(query, w_q, batch, seq_len, self.d_model)?;
        let k_proj = self.linear_projection(key, w_k, batch, seq_len, self.d_model)?;
        let v_proj = self.linear_projection(value, w_v, batch, seq_len, self.d_model)?;

        // Step 2: Split into heads and reshape
        // [batch, seq_len, d_model] → [batch * num_heads, seq_len, d_k]
        let q_heads = self.split_heads(&q_proj, batch, seq_len)?;
        let k_heads = self.split_heads(&k_proj, batch, seq_len)?;
        let v_heads = self.split_heads(&v_proj, batch, seq_len)?;

        // Step 3: Scaled dot-product attention for each head
        let batch_heads = batch * self.num_heads;
        let (head_output, _weights) = self
            .attention
            .execute(
                &q_heads,
                &k_heads,
                &v_heads,
                mask,
                batch_heads,
                seq_len,
                self.d_k,
                self.d_v,
            )
            .await?;

        // Step 4: Concatenate heads
        // [batch * num_heads, seq_len, d_v] → [batch, seq_len, d_model]
        let concat = self.concat_heads(&head_output, batch, seq_len)?;

        // Step 5: Output projection
        let output = self.linear_projection(&concat, w_o, batch, seq_len, self.d_model)?;

        Ok(output)
    }

    /// Linear projection: X @ W
    fn linear_projection(
        &self,
        input: &[f32],
        weight: &[f32],
        batch: u32,
        seq_len: u32,
        d: u32,
    ) -> Result<Vec<f32>> {
        let mut output = vec![0.0f32; (batch * seq_len * d) as usize];

        // Naive matrix multiplication (CPU)
        // In production, use GPU-accelerated MatMul operation
        for b in 0..batch {
            for s in 0..seq_len {
                for out_dim in 0..d {
                    let mut sum = 0.0f32;
                    for in_dim in 0..d {
                        let input_idx = ((b * seq_len + s) * d + in_dim) as usize;
                        let weight_idx = (in_dim * d + out_dim) as usize;
                        sum += input[input_idx] * weight[weight_idx];
                    }
                    let output_idx = ((b * seq_len + s) * d + out_dim) as usize;
                    output[output_idx] = sum;
                }
            }
        }

        Ok(output)
    }

    /// Split heads: [batch, seq_len, d_model] → [batch * num_heads, seq_len, d_k]
    fn split_heads(&self, input: &[f32], batch: u32, seq_len: u32) -> Result<Vec<f32>> {
        let mut output = vec![0.0f32; (batch * self.num_heads * seq_len * self.d_k) as usize];

        for b in 0..batch {
            for s in 0..seq_len {
                for h in 0..self.num_heads {
                    for d in 0..self.d_k {
                        let input_idx =
                            ((b * seq_len + s) * self.d_model + h * self.d_k + d) as usize;
                        let output_idx =
                            (((b * self.num_heads + h) * seq_len + s) * self.d_k + d) as usize;
                        output[output_idx] = input[input_idx];
                    }
                }
            }
        }

        Ok(output)
    }

    /// Concatenate heads: [batch * num_heads, seq_len, d_v] → [batch, seq_len, d_model]
    fn concat_heads(&self, input: &[f32], batch: u32, seq_len: u32) -> Result<Vec<f32>> {
        let mut output = vec![0.0f32; (batch * seq_len * self.d_model) as usize];

        for b in 0..batch {
            for s in 0..seq_len {
                for h in 0..self.num_heads {
                    for d in 0..self.d_v {
                        let input_idx =
                            (((b * self.num_heads + h) * seq_len + s) * self.d_v + d) as usize;
                        let output_idx =
                            ((b * seq_len + s) * self.d_model + h * self.d_v + d) as usize;
                        output[output_idx] = input[input_idx];
                    }
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod multi_head_attention_tests {
    use super::*;
    use anyhow::Context;

    async fn create_test_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("No GPU adapter found")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("Failed to create device")?;

        Ok((Arc::new(device), Arc::new(queue)))
    }

    #[tokio::test]
    async fn test_multi_head_attention_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = MultiHeadAttention::new(device, queue, 8, 512).await;
            assert!(result.is_ok(), "Failed to create multi-head attention");
        }
    }

    #[tokio::test]
    async fn test_multi_head_invalid_dimensions() {
        if let Ok((device, queue)) = create_test_device().await {
            // d_model not divisible by num_heads
            let result = MultiHeadAttention::new(device, queue, 7, 512).await;
            assert!(result.is_err(), "Should fail with invalid dimensions");
        }
    }

    #[tokio::test]
    async fn test_multi_head_small_input() {
        if let Ok((device, queue)) = create_test_device().await {
            if let Ok(mha) = MultiHeadAttention::new(device, queue, 2, 8).await {
                // Small test: batch=1, seq_len=4, d_model=8, num_heads=2
                let batch = 1;
                let seq_len = 4;
                let d_model = 8;

                let query = vec![1.0f32; (batch * seq_len * d_model) as usize];
                let key = vec![1.0f32; (batch * seq_len * d_model) as usize];
                let value = vec![1.0f32; (batch * seq_len * d_model) as usize];

                // Identity weights (simplified)
                let weights = vec![0.0f32; (d_model * d_model) as usize];

                let result = mha
                    .execute(
                        &query, &key, &value, &weights, &weights, &weights, &weights, None, batch,
                        seq_len,
                    )
                    .await;

                assert!(result.is_ok(), "Multi-head attention execution failed");

                if let Ok(output) = result {
                    assert_eq!(output.len(), (batch * seq_len * d_model) as usize);
                }
            }
        }
    }
}
