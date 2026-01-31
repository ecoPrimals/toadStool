//! Cosine Embedding Loss - Similarity-based loss
//!
//! Learns similar/dissimilar embeddings based on labels.

pub async fn cosine_embedding_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input1: &[f32],
    input2: &[f32],
    target: f32, // +1 for similar, -1 for dissimilar
    margin: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if input1.len() != input2.len() {
        return Err("Inputs must have same length".into());
    }
    
    let mut dot = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;
    
    for i in 0..input1.len() {
        dot += input1[i] * input2[i];
        norm1 += input1[i] * input1[i];
        norm2 += input2[i] * input2[i];
    }
    
    let cosine_sim = dot / (norm1.sqrt() * norm2.sqrt() + 1e-8);
    
    let loss = if target == 1.0 {
        1.0 - cosine_sim
    } else {
        (cosine_sim - margin).max(0.0)
    };
    
    Ok(loss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_cosine_embedding_loss_basic() {
        let dev = get_test_device().await;
        let input1 = vec![1.0, 0.0, 0.0];
        let input2 = vec![0.9, 0.1, 0.0];
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_cosine_embedding_loss_edge_cases() {
        let dev = get_test_device().await;
        
        // Identical embeddings (zero loss for similar pairs)
        let input1 = vec![1.0, 2.0, 3.0];
        let input2 = vec![1.0, 2.0, 3.0];
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!(loss.abs() < 1e-6);
        
        // Orthogonal vectors (cosine similarity = 0)
        let input1 = vec![1.0, 0.0, 0.0];
        let input2 = vec![0.0, 1.0, 0.0];
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!((loss - 1.0).abs() < 0.01); // Loss = 1 - 0 = 1
    }

    #[tokio::test]
    async fn test_cosine_embedding_loss_boundary() {
        let dev = get_test_device().await;
        
        // Dissimilar pairs (target = -1)
        let input1 = vec![1.0, 0.0, 0.0];
        let input2 = vec![0.0, 1.0, 0.0];
        
        // With margin=0.5, dissimilar loss = max(0, cosine_sim - margin)
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, -1.0, 0.5).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
        
        // Similar pairs vs dissimilar pairs
        let input3 = vec![1.0, 1.0, 1.0];
        let input4 = vec![1.0, 1.0, 1.0];
        
        let loss_similar = cosine_embedding_loss(&dev.device, &dev.queue, &input3, &input4, 1.0, 0.0).await.unwrap();
        let loss_dissimilar = cosine_embedding_loss(&dev.device, &dev.queue, &input3, &input4, -1.0, 0.0).await.unwrap();
        
        // Similar should be near zero, dissimilar should be higher
        assert!(loss_similar < 0.01);
        assert!(loss_dissimilar > loss_similar);
    }

    #[tokio::test]
    async fn test_cosine_embedding_loss_large_batch() {
        let dev = get_test_device().await;
        
        // Large embedding dimension
        let dim = 128;
        let input1: Vec<f32> = (0..dim).map(|i| (i % 10) as f32 * 0.1).collect();
        let input2: Vec<f32> = (0..dim).map(|i| ((i + 1) % 10) as f32 * 0.1).collect();
        
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        
        assert!(loss.is_finite());
        assert!(loss >= 0.0);
    }

    #[tokio::test]
    async fn test_cosine_embedding_loss_precision() {
        let dev = get_test_device().await;
        
        // Test with known cosine similarity
        let input1 = vec![1.0, 0.0];  // Unit vector along x
        let input2 = vec![0.0, 1.0];  // Unit vector along y
        
        // Cosine similarity = 0 (perpendicular)
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!((loss - 1.0).abs() < 0.01); // Loss = 1 - 0 = 1
        
        // Opposite vectors (cosine similarity = -1)
        let input1 = vec![1.0, 0.0];
        let input2 = vec![-1.0, 0.0];
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!((loss - 2.0).abs() < 0.01); // Loss = 1 - (-1) = 2
    }
}
