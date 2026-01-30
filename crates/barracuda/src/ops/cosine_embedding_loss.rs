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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_cosine_embedding_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input1 = vec![1.0, 0.0, 0.0];
        let input2 = vec![0.9, 0.1, 0.0];
        let loss = cosine_embedding_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!(loss >= 0.0);
    }
}
