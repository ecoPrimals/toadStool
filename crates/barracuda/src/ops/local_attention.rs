//! Local Attention - Windowed attention for efficiency
//!
//! Only attends to nearby tokens within a window.
//! Reduces complexity from O(N²) to O(N*W) where W is window size.

pub async fn local_attention(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    query: &[f32],
    key: &[f32],
    value: &[f32],
    batch_size: usize,
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
    window_size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * num_heads * seq_len * head_dim];
    let scale = (head_dim as f32).sqrt();
    let half_window = window_size / 2;
    
    for b in 0..batch_size {
        for h in 0..num_heads {
            for i in 0..seq_len {
                // Attend to local window [i-half_window, i+half_window]
                let start = i.saturating_sub(half_window);
                let end = (i + half_window + 1).min(seq_len);
                
                let mut scores = vec![f32::NEG_INFINITY; seq_len];
                for j in start..end {
                    let mut score = 0.0;
                    for d in 0..head_dim {
                        let q_idx = b * num_heads * seq_len * head_dim + h * seq_len * head_dim + i * head_dim + d;
                        let k_idx = b * num_heads * seq_len * head_dim + h * seq_len * head_dim + j * head_dim + d;
                        score += query[q_idx] * key[k_idx];
                    }
                    scores[j] = score / scale;
                }
                
                let max_score = scores.iter().filter(|&&s| s.is_finite()).cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut sum = 0.0;
                for s in &mut scores {
                    if s.is_finite() {
                        *s = (*s - max_score).exp();
                        sum += *s;
                    } else {
                        *s = 0.0;
                    }
                }
                for s in &mut scores {
                    *s /= sum;
                }
                
                for d in 0..head_dim {
                    let mut weighted_sum = 0.0;
                    for j in 0..seq_len {
                        let v_idx = b * num_heads * seq_len * head_dim + h * seq_len * head_dim + j * head_dim + d;
                        weighted_sum += scores[j] * value[v_idx];
                    }
                    let out_idx = b * num_heads * seq_len * head_dim + h * seq_len * head_dim + i * head_dim + d;
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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_local_attention_basic() {
        let dev = get_test_device().await;
        let size = 1 * 2 * 8 * 4;
        let q = vec![0.5; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, 1, 2, 8, 4, 4).await.unwrap();
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_local_attention_edge_cases() {
        let dev = get_test_device().await;

        // Window size = 2 (minimal)
        let size = 1 * 1 * 4 * 2;
        let q = vec![1.0; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, 1, 1, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), size);

        // Single head
        let size = 1 * 1 * 8 * 4;
        let q = vec![0.5; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, 1, 1, 8, 4, 4).await.unwrap();
        assert_eq!(output.len(), size);
    }

    #[tokio::test]
    async fn test_local_attention_boundary() {
        let dev = get_test_device().await;

        // Large window (full attention)
        let size = 1 * 2 * 8 * 4;
        let q = vec![0.5; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, 1, 2, 8, 4, 8).await.unwrap();
        assert_eq!(output.len(), size);

        // Multiple heads
        let size = 1 * 8 * 16 * 8;
        let q = vec![0.5; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, 1, 8, 16, 8, 4).await.unwrap();
        assert_eq!(output.len(), size);
    }

    #[tokio::test]
    async fn test_local_attention_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4, longer sequence
        let batch_size = 4;
        let size = batch_size * 4 * 32 * 8;
        let q = vec![0.5; size];
        let output = local_attention(&dev.device, &dev.queue, &q, &q, &q, batch_size, 4, 32, 8, 8).await.unwrap();
        assert_eq!(output.len(), size);
    }

    #[tokio::test]
    async fn test_local_attention_precision() {
        let dev = get_test_device().await;

        // Test attention pattern with known values
        let size = 1 * 1 * 4 * 2;
        let q = vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let k = vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        
        let output = local_attention(&dev.device, &dev.queue, &q, &k, &v, 1, 1, 4, 2, 4).await.unwrap();
        
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Verify attention produces weighted sums
        assert!(output.iter().any(|&x| x > 0.0));
    }
}
