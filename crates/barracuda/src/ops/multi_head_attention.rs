//! Multi-Head Attention - Complete attention layer with projections
//!
//! ## Deep Debt Principles
//!
//! - **Self-contained**: Includes input/output projections
//! - **Production-ready**: Correct implementation of full MHA layer
//! - **Evolution path**: GPU-optimized implementation noted
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

/// Multi-head attention with projections
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::multi_head_attention::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let batch = 2;
/// let seq_len = 128;
/// let d_model = 512;
/// let num_heads = 8;
///
/// let input_size = batch * seq_len * d_model;
/// let query = vec![0.5; input_size];
/// let key = query.clone();
/// let value = query.clone();
///
/// // Initialize projection weights (in practice, from trained model)
/// let w_q = vec![0.01; d_model * d_model];
/// let w_k = w_q.clone();
/// let w_v = w_q.clone();
/// let w_o = w_q.clone();
///
/// let output = multi_head_attention(
///     device, queue,
///     &query, &key, &value,
///     &w_q, &w_k, &w_v, &w_o,
///     batch, seq_len, d_model, num_heads
/// ).await.unwrap();
/// # }
/// ```
///
/// ## Deep Debt Note
///
/// Current: CPU reference implementation demonstrating algorithm
/// Evolution: GPU implementation with fused operations for performance
pub async fn multi_head_attention(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    query: &[f32],
    key: &[f32],
    value: &[f32],
    w_q: &[f32],  // Query projection [d_model, d_model]
    w_k: &[f32],  // Key projection [d_model, d_model]
    w_v: &[f32],  // Value projection [d_model, d_model]
    w_o: &[f32],  // Output projection [d_model, d_model]
    batch_size: usize,
    seq_len: usize,
    d_model: usize,
    num_heads: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if d_model % num_heads != 0 {
        return Err("d_model must be divisible by num_heads".into());
    }
    
    let head_dim = d_model / num_heads;
    let input_size = batch_size * seq_len * d_model;
    
    // Validate dimensions
    if query.len() != input_size || key.len() != input_size || value.len() != input_size {
        return Err(format!(
            "Input dimension mismatch: expected {}, got Q:{}, K:{}, V:{}",
            input_size, query.len(), key.len(), value.len()
        ).into());
    }
    
    if w_q.len() != d_model * d_model {
        return Err(format!("Weight dimension mismatch: w_q expected {}, got {}", 
            d_model * d_model, w_q.len()).into());
    }
    
    // CPU implementation (reference)
    // Deep Debt Evolution: Replace with fused GPU kernels
    
    // Step 1: Project Q, K, V through their respective weight matrices
    let mut q_proj = vec![0.0f32; input_size];
    let mut k_proj = vec![0.0f32; input_size];
    let mut v_proj = vec![0.0f32; input_size];
    
    for b in 0..batch_size {
        for s in 0..seq_len {
            // Project each position
            for i in 0..d_model {
                let mut q_sum = 0.0;
                let mut k_sum = 0.0;
                let mut v_sum = 0.0;
                
                for j in 0..d_model {
                    let input_idx = b * seq_len * d_model + s * d_model + j;
                    q_sum += query[input_idx] * w_q[j * d_model + i];
                    k_sum += key[input_idx] * w_k[j * d_model + i];
                    v_sum += value[input_idx] * w_v[j * d_model + i];
                }
                
                let out_idx = b * seq_len * d_model + s * d_model + i;
                q_proj[out_idx] = q_sum;
                k_proj[out_idx] = k_sum;
                v_proj[out_idx] = v_sum;
            }
        }
    }
    
    // Step 2: Apply scaled dot-product attention for each head
    let mut concat_heads = vec![0.0f32; input_size];
    let scale = (head_dim as f32).sqrt();
    
    for b in 0..batch_size {
        for h in 0..num_heads {
            // Compute attention for this head
            for i in 0..seq_len {
                // Compute attention scores
                let mut scores = vec![0.0f32; seq_len];
                
                for j in 0..seq_len {
                    let mut score = 0.0;
                    
                    for d in 0..head_dim {
                        let q_idx = b * seq_len * d_model + i * d_model + h * head_dim + d;
                        let k_idx = b * seq_len * d_model + j * d_model + h * head_dim + d;
                        score += q_proj[q_idx] * k_proj[k_idx];
                    }
                    
                    scores[j] = score / scale;
                }
                
                // Softmax
                let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut sum = 0.0;
                for s in &mut scores {
                    *s = (*s - max_score).exp();
                    sum += *s;
                }
                for s in &mut scores {
                    *s /= sum;
                }
                
                // Apply to values
                for d in 0..head_dim {
                    let mut weighted_sum = 0.0;
                    
                    for j in 0..seq_len {
                        let v_idx = b * seq_len * d_model + j * d_model + h * head_dim + d;
                        weighted_sum += scores[j] * v_proj[v_idx];
                    }
                    
                    let out_idx = b * seq_len * d_model + i * d_model + h * head_dim + d;
                    concat_heads[out_idx] = weighted_sum;
                }
            }
        }
    }
    
    // Step 3: Project concatenated heads through output matrix
    let mut output = vec![0.0f32; input_size];
    
    for b in 0..batch_size {
        for s in 0..seq_len {
            for i in 0..d_model {
                let mut sum = 0.0;
                
                for j in 0..d_model {
                    let concat_idx = b * seq_len * d_model + s * d_model + j;
                    sum += concat_heads[concat_idx] * w_o[j * d_model + i];
                }
                
                let out_idx = b * seq_len * d_model + s * d_model + i;
                output[out_idx] = sum;
            }
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_multi_head_attention_dimensions() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let batch = 1;
        let seq_len = 4;
        let d_model = 8;
        let num_heads = 2;
        
        let input_size = batch * seq_len * d_model;
        let weight_size = d_model * d_model;
        
        let query = vec![0.5; input_size];
        let key = query.clone();
        let value = query.clone();
        
        let w_q = vec![0.01; weight_size];
        let w_k = w_q.clone();
        let w_v = w_q.clone();
        let w_o = w_q.clone();
        
        let output = multi_head_attention(
            &device, &queue,
            &query, &key, &value,
            &w_q, &w_k, &w_v, &w_o,
            batch, seq_len, d_model, num_heads
        ).await.unwrap();
        
        assert_eq!(output.len(), input_size);
    }
}
