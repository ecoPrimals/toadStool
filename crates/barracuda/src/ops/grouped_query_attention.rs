//! Grouped Query Attention - LLaMA-style efficient attention
//!
//! ## Algorithm
//!
//! Multi-Query Attention variant where queries have multiple heads
//! but keys/values share heads across groups.
//!
//! Memory efficient: Reduces KV cache size for inference.
//! Reference: LLaMA, LLaMA-2 (Meta AI)

pub async fn grouped_query_attention(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    query: &[f32],
    key: &[f32],
    value: &[f32],
    batch_size: usize,
    num_q_heads: usize,
    num_kv_heads: usize,
    seq_len: usize,
    head_dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if num_q_heads % num_kv_heads != 0 {
        return Err("num_q_heads must be divisible by num_kv_heads".into());
    }

    let heads_per_group = num_q_heads / num_kv_heads;
    let mut output = vec![0.0f32; batch_size * num_q_heads * seq_len * head_dim];
    let scale = (head_dim as f32).sqrt();

    for b in 0..batch_size {
        for qh in 0..num_q_heads {
            let kv_head = qh / heads_per_group;

            for i in 0..seq_len {
                let mut scores = vec![0.0f32; seq_len];
                for j in 0..seq_len {
                    let mut score = 0.0;
                    for d in 0..head_dim {
                        let q_idx = b * num_q_heads * seq_len * head_dim
                            + qh * seq_len * head_dim
                            + i * head_dim
                            + d;
                        let k_idx = b * num_kv_heads * seq_len * head_dim
                            + kv_head * seq_len * head_dim
                            + j * head_dim
                            + d;
                        score += query[q_idx] * key[k_idx];
                    }
                    scores[j] = score / scale;
                }

                let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut sum = 0.0;
                for s in &mut scores {
                    *s = (*s - max_score).exp();
                    sum += *s;
                }
                for s in &mut scores {
                    *s /= sum;
                }

                for d in 0..head_dim {
                    let mut weighted_sum = 0.0;
                    for j in 0..seq_len {
                        let v_idx = b * num_kv_heads * seq_len * head_dim
                            + kv_head * seq_len * head_dim
                            + j * head_dim
                            + d;
                        weighted_sum += scores[j] * value[v_idx];
                    }
                    let out_idx = b * num_q_heads * seq_len * head_dim
                        + qh * seq_len * head_dim
                        + i * head_dim
                        + d;
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
    async fn test_grouped_query_attention() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let q = vec![0.5; 1 * 8 * 4 * 4]; // 8 query heads
        let k = vec![0.5; 1 * 2 * 4 * 4]; // 2 kv heads (4 heads per group)
        let v = k.clone();
        let output = grouped_query_attention(&dev.device, &dev.queue, &q, &k, &v, 1, 8, 2, 4, 4)
            .await
            .unwrap();
        assert_eq!(output.len(), q.len());
    }
}
