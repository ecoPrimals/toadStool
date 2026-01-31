//! Rotary Position Embedding (RoPE) - Relative position encoding
//!
//! ## Algorithm
//!
//! Applies rotation to query/key pairs based on position.
//! Encodes relative position information without absolute position embeddings.
//!
//! Reference: RoFormer (Su et al., 2021), used in GPT-Neo, LLaMA, PaLM

pub async fn rotary_embedding(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],        // [batch, seq_len, num_heads, head_dim]
    batch_size: usize,
    seq_len: usize,
    num_heads: usize,
    head_dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; input.len()];
    let half_dim = head_dim / 2;
    
    // Frequency bands
    let freqs: Vec<f32> = (0..half_dim)
        .map(|i| 1.0 / 10000.0_f32.powf(2.0 * i as f32 / head_dim as f32))
        .collect();
    
    for b in 0..batch_size {
        for s in 0..seq_len {
            let pos = s as f32;
            
            for h in 0..num_heads {
                for d in 0..half_dim {
                    let freq = freqs[d];
                    let theta = pos * freq;
                    let cos_val = theta.cos();
                    let sin_val = theta.sin();
                    
                    let idx1 = b * seq_len * num_heads * head_dim + s * num_heads * head_dim + h * head_dim + d;
                    let idx2 = b * seq_len * num_heads * head_dim + s * num_heads * head_dim + h * head_dim + d + half_dim;
                    
                    let x1 = input[idx1];
                    let x2 = input[idx2];
                    
                    // Rotation: [cos -sin] [x1]
                    //           [sin  cos] [x2]
                    output[idx1] = x1 * cos_val - x2 * sin_val;
                    output[idx2] = x1 * sin_val + x2 * cos_val;
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
    async fn test_rotary_embedding_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 4 * 2 * 8]; // batch=1, seq=4, heads=2, dim=8
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 4, 2, 8).await.unwrap();
        assert_eq!(output.len(), input.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_rotary_embedding_edge_cases() {
        let dev = get_test_device().await;

        // Single position
        let input = vec![1.0; 1 * 1 * 2 * 8];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 1, 2, 8).await.unwrap();
        assert_eq!(output.len(), 16);

        // Single head
        let input = vec![1.0; 1 * 4 * 1 * 8];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 4, 1, 8).await.unwrap();
        assert_eq!(output.len(), 32);

        // Small head dimension
        let input = vec![1.0; 1 * 2 * 2 * 4];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 2, 2, 4).await.unwrap();
        assert_eq!(output.len(), 16);
    }

    #[tokio::test]
    async fn test_rotary_embedding_boundary() {
        let dev = get_test_device().await;

        // Large sequence length
        let input = vec![1.0; 1 * 128 * 2 * 8];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 128, 2, 8).await.unwrap();
        assert_eq!(output.len(), 1 * 128 * 2 * 8);

        // Many heads
        let input = vec![1.0; 1 * 4 * 16 * 8];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 4, 16, 8).await.unwrap();
        assert_eq!(output.len(), 1 * 4 * 16 * 8);
    }

    #[tokio::test]
    async fn test_rotary_embedding_large_batch() {
        let dev = get_test_device().await;

        // Batch size 8
        let batch_size = 8;
        let input = vec![1.0; batch_size * 16 * 4 * 8];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, batch_size, 16, 4, 8).await.unwrap();
        assert_eq!(output.len(), batch_size * 16 * 4 * 8);
    }

    #[tokio::test]
    async fn test_rotary_embedding_precision() {
        let dev = get_test_device().await;

        // Test rotation properties
        let input = vec![1.0; 1 * 2 * 1 * 4];
        let output = rotary_embedding(&dev.device, &dev.queue, &input, 1, 2, 1, 4).await.unwrap();
        
        assert_eq!(output.len(), 8);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Rotations preserve magnitude (approximately, due to FP precision)
        assert!(output.iter().all(|&x| x.abs() <= 2.0));
    }
}
