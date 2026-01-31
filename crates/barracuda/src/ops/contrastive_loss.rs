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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_contrastive_loss_basic() {
        let dev = get_test_device().await;
        let embeddings = vec![0.5; 4 * 8]; // 2 batches (4 samples), 8 dims
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, 2, 8).await.unwrap();
        assert!(loss.is_finite());
        assert!(loss >= 0.0);
    }

    #[tokio::test]
    async fn test_contrastive_loss_edge_cases() {
        let dev = get_test_device().await;
        
        // Single batch (2 samples forming 1 positive pair)
        let embeddings = vec![
            1.0, 0.0, 0.0, 0.0,  // Sample 0
            1.0, 0.0, 0.0, 0.0,  // Sample 1 (positive pair)
        ];
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, 1, 4).await.unwrap();
        assert!(loss.is_finite());
        
        // Orthogonal embeddings
        let embeddings = vec![
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, 2, 4).await.unwrap();
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_contrastive_loss_boundary() {
        let dev = get_test_device().await;
        
        // Different temperatures should produce finite losses
        let embeddings = vec![0.7; 6 * 10]; // 3 batches, 10 dims
        
        let loss1 = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.1, 3, 10).await.unwrap();
        let loss2 = contrastive_loss(&dev.device, &dev.queue, &embeddings, 1.0, 3, 10).await.unwrap();
        
        assert!(loss1.is_finite());
        assert!(loss2.is_finite());
        assert!(loss1 >= 0.0);
        assert!(loss2 >= 0.0);
        
        // Test with varied embeddings for better temperature sensitivity
        let mut embeddings2 = vec![0.0; 4 * 10];
        for i in 0..4 {
            for j in 0..10 {
                embeddings2[i * 10 + j] = (i as f32 + j as f32) * 0.1;
            }
        }
        let loss3 = contrastive_loss(&dev.device, &dev.queue, &embeddings2, 0.5, 2, 10).await.unwrap();
        assert!(loss3.is_finite());
    }

    #[tokio::test]
    async fn test_contrastive_loss_large_batch() {
        let dev = get_test_device().await;
        
        // Large batch
        let batch_size = 16;
        let embed_dim = 128;
        
        let embeddings: Vec<f32> = (0..batch_size * 2 * embed_dim)
            .map(|i| ((i % 100) as f32) / 100.0)
            .collect();
        
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, batch_size, embed_dim).await.unwrap();
        
        assert!(loss.is_finite());
        assert!(loss >= 0.0);
    }

    #[tokio::test]
    async fn test_contrastive_loss_precision() {
        let dev = get_test_device().await;
        
        // Test with known similar/dissimilar pairs
        let embeddings = vec![
            1.0, 0.0,  // Sample 0 (similar to sample 2)
            0.0, 1.0,  // Sample 1
            0.9, 0.1,  // Sample 2 (positive pair with sample 0)
            0.1, 0.9,  // Sample 3 (positive pair with sample 1)
        ];
        
        let loss = contrastive_loss(&dev.device, &dev.queue, &embeddings, 0.5, 2, 2).await.unwrap();
        
        // Loss should be finite and positive
        assert!(loss.is_finite());
        assert!(loss > 0.0);
        
        // Loss should be reasonable (not too large)
        assert!(loss < 100.0);
    }
}
