//! Contrastive Loss - SimCLR, MoCo style
//!
//! Learns representations by contrasting positive/negative pairs.
//! Used in self-supervised learning.

pub async fn contrastive_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    embeddings: &[f32],  // [batch * 2, embed_dim] (positive pairs concatenated)
    temperature: f32,
    batch_size: usize,
    embed_dim: usize,
) -> Result<f32, Box<dyn std::error::Error>> {
    let total_size = batch_size * 2;
    
    // Compute similarity matrix
    let mut similarities = vec![0.0f32; total_size * total_size];
    
    for i in 0..total_size {
        for j in 0..total_size {
            let mut dot_product = 0.0;
            let mut norm_i = 0.0;
            let mut norm_j = 0.0;
            
            for d in 0..embed_dim {
                let emb_i = embeddings[i * embed_dim + d];
                let emb_j = embeddings[j * embed_dim + d];
                dot_product += emb_i * emb_j;
                norm_i += emb_i * emb_i;
                norm_j += emb_j * emb_j;
            }
            
            let cosine_sim = dot_product / (norm_i.sqrt() * norm_j.sqrt() + 1e-8);
            similarities[i * total_size + j] = cosine_sim / temperature;
        }
    }
    
    // Contrastive loss: pull positives together, push negatives apart
    let mut loss = 0.0;
    
    for i in 0..batch_size {
        let pos_pair_idx = (i + batch_size) % total_size;
        
        // Numerator: similarity to positive
        let pos_sim = similarities[i * total_size + pos_pair_idx].exp();
        
        // Denominator: sum of all similarities except self
        let mut denom = 0.0;
        for j in 0..total_size {
            if j != i {
                denom += similarities[i * total_size + j].exp();
            }
        }
        
        loss += -(pos_sim / denom).ln();
    }
    
    Ok(loss / batch_size as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_contrastive_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let embeddings = vec![0.5; 4 * 8]; // 2 batches (4 samples), 8 dims
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, 2, 8).await.unwrap();
        assert!(loss.is_finite());
    }
}
