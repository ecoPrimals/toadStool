//! ALiBi Position Encoding - Attention with Linear Biases
//!
//! Adds position-dependent bias to attention scores.
//! No position embeddings needed - bias encodes position directly.
//!
//! Reference: "Train Short, Test Long" (Press et al., 2021)

pub async fn alibi_position(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    attention_scores: &[f32], // [batch, heads, seq_len, seq_len]
    batch_size: usize,
    num_heads: usize,
    seq_len: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = attention_scores.to_vec();
    
    // Head-specific slopes (geometric sequence)
    let slopes: Vec<f32> = (0..num_heads)
        .map(|h| 2.0_f32.powf(-(8.0 * (h + 1) as f32 / num_heads as f32)))
        .collect();
    
    for b in 0..batch_size {
        for h in 0..num_heads {
            let slope = slopes[h];
            
            for i in 0..seq_len {
                for j in 0..seq_len {
                    let distance = (i as isize - j as isize).abs() as f32;
                    let bias = -slope * distance;
                    
                    let idx = b * num_heads * seq_len * seq_len + h * seq_len * seq_len + i * seq_len + j;
                    output[idx] += bias;
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
    async fn test_alibi_position() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let scores = vec![1.0; 1 * 2 * 4 * 4]; // batch=1, heads=2, seq=4
        let output = alibi_position(&dev.device, &dev.queue, &scores, 1, 2, 4).await.unwrap();
        assert_eq!(output.len(), scores.len());
    }
}
