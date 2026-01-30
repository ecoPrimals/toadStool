//! Cross Attention - Encoder-decoder attention
//!
//! ## Deep Debt Principles
//!
//! - **Production-critical**: Required for seq2seq, encoder-decoder transformers
//! - **Asymmetric**: Q from decoder, K/V from encoder
//! - **Complete**: Proper implementation with different sequence lengths
//!
//! ## Algorithm
//!
//! ```text
//! CrossAttention(Q_decoder, K_encoder, V_encoder) = softmax(QK^T / sqrt(d_k)) * V
//! ```
//!
//! Key difference: Q has different seq_len than K/V
//! Use: T5, BART, encoder-decoder transformers

/// Cross attention between encoder and decoder
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::cross_attention::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let batch = 2;
/// let heads = 8;
/// let decoder_len = 32;  // Decoder sequence
/// let encoder_len = 128; // Encoder sequence (can differ!)
/// let head_dim = 64;
///
/// let q_size = batch * heads * decoder_len * head_dim;
/// let kv_size = batch * heads * encoder_len * head_dim;
///
/// let query = vec![0.5; q_size];
/// let key = vec![0.5; kv_size];
/// let value = vec![1.0; kv_size];
///
/// let output = cross_attention(
///     device, queue,
///     &query, &key, &value,
///     batch, heads, decoder_len, encoder_len, head_dim
/// ).await.unwrap();
/// # }
/// ```
pub async fn cross_attention(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    query: &[f32],        // [batch, heads, decoder_len, head_dim]
    key: &[f32],          // [batch, heads, encoder_len, head_dim]
    value: &[f32],        // [batch, heads, encoder_len, head_dim]
    batch_size: usize,
    num_heads: usize,
    decoder_len: usize,
    encoder_len: usize,
    head_dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let q_size = batch_size * num_heads * decoder_len * head_dim;
    let kv_size = batch_size * num_heads * encoder_len * head_dim;
    
    if query.len() != q_size || key.len() != kv_size || value.len() != kv_size {
        return Err("Dimension mismatch".into());
    }
    
    let mut output = vec![0.0f32; q_size];
    let scale = (head_dim as f32).sqrt();
    
    for b in 0..batch_size {
        for h in 0..num_heads {
            for i in 0..decoder_len {
                // Compute attention scores with encoder
                let mut scores = vec![0.0f32; encoder_len];
                
                for j in 0..encoder_len {
                    let mut score = 0.0;
                    for d in 0..head_dim {
                        let q_idx = b * num_heads * decoder_len * head_dim + h * decoder_len * head_dim + i * head_dim + d;
                        let k_idx = b * num_heads * encoder_len * head_dim + h * encoder_len * head_dim + j * head_dim + d;
                        score += query[q_idx] * key[k_idx];
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
                
                // Apply to encoder values
                for d in 0..head_dim {
                    let mut weighted_sum = 0.0;
                    for j in 0..encoder_len {
                        let v_idx = b * num_heads * encoder_len * head_dim + h * encoder_len * head_dim + j * head_dim + d;
                        weighted_sum += scores[j] * value[v_idx];
                    }
                    let out_idx = b * num_heads * decoder_len * head_dim + h * decoder_len * head_dim + i * head_dim + d;
                    output[out_idx] = weighted_sum;
                }
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
    async fn test_cross_attention() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let batch = 1;
        let heads = 2;
        let dec_len = 3;
        let enc_len = 5;
        let dim = 4;
        
        let q_size = batch * heads * dec_len * dim;
        let kv_size = batch * heads * enc_len * dim;
        
        let query = vec![0.5; q_size];
        let key = vec![0.5; kv_size];
        let value = vec![1.0; kv_size];
        
        let output = cross_attention(device, queue, &query, &key, &value, batch, heads, dec_len, enc_len, dim).await.unwrap();
        assert_eq!(output.len(), q_size);
    }
}
