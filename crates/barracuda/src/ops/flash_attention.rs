//! Flash Attention - Memory-efficient attention (O(N) memory)
//!
//! ## Deep Debt Principles
//!
//! - **Memory-efficient**: O(N) memory vs O(N²) for standard attention
//! - **Production-critical**: Enables long sequences (512+ tokens)
//! - **Modern research**: FlashAttention-2 algorithm (Dao et al., 2023)
//!
//! ## Algorithm
//!
//! Tiled computation that avoids materializing full N×N attention matrix:
//! - Process attention in blocks
//! - Online softmax with rescaling
//! - Reduces memory from O(N²) to O(N)
//!
//! Reference: "FlashAttention: Fast and Memory-Efficient Exact Attention with IO-Awareness"
//! https://arxiv.org/abs/2205.14135

/// Flash Attention - Memory-efficient implementation
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::flash_attention::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let batch = 2;
/// let num_heads = 8;
/// let seq_len = 2048; // Long sequences possible!
/// let head_dim = 64;
///
/// let total_size = batch * num_heads * seq_len * head_dim;
/// let query = vec![0.5; total_size];
/// let key = vec![0.5; total_size];
/// let value = vec![1.0; total_size];
///
/// let output = flash_attention(
///     device, queue,
///     &query, &key, &value,
///     batch, num_heads, seq_len, head_dim
/// ).await.unwrap();
/// # }
/// ```
///
/// ## Deep Debt Note
///
/// Current: Reference implementation using tiling strategy
/// Evolution: GPU kernel with optimal block sizes for hardware
/// Memory: O(N) complexity enables sequences of 2048+ tokens
pub async fn flash_attention(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    query: &[f32],
    key: &[f32],
    value: &[f32],
    batch_size: usize,
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let expected_size = batch_size * num_heads * seq_len * head_dim;
    if query.len() != expected_size || key.len() != expected_size || value.len() != expected_size {
        return Err(format!(
            "Dimension mismatch: expected {} elements",
            expected_size
        ).into());
    }
    
    // Flash Attention: Tiled computation
    // Block size for tiling (tunable for hardware)
    const BLOCK_SIZE: usize = 64;
    
    let mut output = vec![0.0f32; expected_size];
    let scale = (head_dim as f32).sqrt();
    
    for b in 0..batch_size {
        for h in 0..num_heads {
            // Process in tiles to reduce memory
            for q_block_start in (0..seq_len).step_by(BLOCK_SIZE) {
                let q_block_end = (q_block_start + BLOCK_SIZE).min(seq_len);
                
                // Running statistics for online softmax
                let mut row_max = vec![f32::NEG_INFINITY; q_block_end - q_block_start];
                let mut row_sum = vec![0.0f32; q_block_end - q_block_start];
                let mut acc_output = vec![0.0f32; (q_block_end - q_block_start) * head_dim];
                
                // Process key/value blocks
                for kv_block_start in (0..seq_len).step_by(BLOCK_SIZE) {
                    let kv_block_end = (kv_block_start + BLOCK_SIZE).min(seq_len);
                    
                    // Compute scores for this block
                    for (local_i, i) in (q_block_start..q_block_end).enumerate() {
                        for j in kv_block_start..kv_block_end {
                            let mut score = 0.0;
                            
                            for d in 0..head_dim {
                                let q_idx = b * num_heads * seq_len * head_dim
                                          + h * seq_len * head_dim
                                          + i * head_dim + d;
                                let k_idx = b * num_heads * seq_len * head_dim
                                          + h * seq_len * head_dim
                                          + j * head_dim + d;
                                score += query[q_idx] * key[k_idx];
                            }
                            score /= scale;
                            
                            // Online softmax: Update running max
                            let old_max = row_max[local_i];
                            let new_max = old_max.max(score);
                            
                            if new_max > old_max {
                                // Rescale previous accumulations
                                let rescale = (old_max - new_max).exp();
                                row_sum[local_i] *= rescale;
                                for d in 0..head_dim {
                                    acc_output[local_i * head_dim + d] *= rescale;
                                }
                                row_max[local_i] = new_max;
                            }
                            
                            // Add contribution from this score
                            let exp_score = (score - new_max).exp();
                            row_sum[local_i] += exp_score;
                            
                            for d in 0..head_dim {
                                let v_idx = b * num_heads * seq_len * head_dim
                                          + h * seq_len * head_dim
                                          + j * head_dim + d;
                                acc_output[local_i * head_dim + d] += exp_score * value[v_idx];
                            }
                        }
                    }
                }
                
                // Normalize and write output
                for (local_i, i) in (q_block_start..q_block_end).enumerate() {
                    for d in 0..head_dim {
                        let out_idx = b * num_heads * seq_len * head_dim
                                    + h * seq_len * head_dim
                                    + i * head_dim + d;
                        output[out_idx] = acc_output[local_i * head_dim + d] / row_sum[local_i];
                    }
                }
            }
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_flash_attention_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Small test case
        let batch = 1;
        let heads = 1;
        let seq = 4;
        let dim = 4;
        
        let size = batch * heads * seq * dim;
        let query = vec![0.5; size];
        let key = query.clone();
        let value = vec![1.0; size];
        
        let output = flash_attention(
            device, queue,
            &query, &key, &value,
            batch, heads, seq, dim
        ).await.unwrap();
        
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_flash_attention_edge_cases() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Single token (seq_len = 1)
        let batch = 1;
        let heads = 2;
        let seq = 1;
        let dim = 8;
        
        let size = batch * heads * seq * dim;
        let query = vec![1.0; size];
        let key = query.clone();
        let value = vec![2.0; size];
        
        let output = flash_attention(
            device, queue,
            &query, &key, &value,
            batch, heads, seq, dim
        ).await.unwrap();
        
        assert_eq!(output.len(), size);
        // With single token, output should approximate value (attention weight = 1)
        for &val in &output {
            assert!((val - 2.0).abs() < 0.1);
        }
    }

    #[tokio::test]
    async fn test_flash_attention_boundary() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Test block boundary (seq_len = 64, exactly one block)
        let batch = 1;
        let heads = 1;
        let seq = 64;
        let dim = 16;
        
        let size = batch * heads * seq * dim;
        let query = vec![0.1; size];
        let key = vec![0.2; size];
        let value = vec![1.0; size];
        
        let output = flash_attention(
            device, queue,
            &query, &key, &value,
            batch, heads, seq, dim
        ).await.unwrap();
        
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite() && x > 0.0));
    }

    #[tokio::test]
    async fn test_flash_attention_large_sequence() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Long sequence (tests tiling across multiple blocks)
        let batch = 2;
        let heads = 4;
        let seq = 128; // > BLOCK_SIZE (64), requires tiling
        let dim = 32;
        
        let size = batch * heads * seq * dim;
        let query = vec![0.5; size];
        let key = query.clone();
        let value = vec![1.0; size];
        
        let output = flash_attention(
            device, queue,
            &query, &key, &value,
            batch, heads, seq, dim
        ).await.unwrap();
        
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
        
        // Output should be close to uniform (all queries/keys are same)
        let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;
        assert!((mean - 1.0).abs() < 0.2);
    }

    #[tokio::test]
    async fn test_flash_attention_precision() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        // Test with different head dimensions
        let batch = 1;
        let heads = 2;
        let seq = 8;
        
        for dim in [16, 32, 64] {
            let size = batch * heads * seq * dim;
            let query = vec![0.5; size];
            let key = query.clone();
            let value = vec![1.0; size];
            
            let output = flash_attention(
                device, queue,
                &query, &key, &value,
                batch, heads, seq, dim
            ).await.unwrap();
            
            assert_eq!(output.len(), size);
            assert!(output.iter().all(|&x| x.is_finite()));
            
            // Check attention is normalized (sums to value approximately)
            let chunk_sum: f32 = output[0..dim].iter().sum();
            assert!((chunk_sum - dim as f32).abs() / (dim as f32) < 0.2);
        }
    }
}
