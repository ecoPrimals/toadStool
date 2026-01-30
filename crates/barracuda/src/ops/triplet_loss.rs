//! Triplet Loss - Learn embeddings with anchor/positive/negative
//!
//! Pulls anchor closer to positive, pushes away from negative.
//! Used in face recognition, metric learning.

pub async fn triplet_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    anchor: &[f32],
    positive: &[f32],
    negative: &[f32],
    margin: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if anchor.len() != positive.len() || anchor.len() != negative.len() {
        return Err("All inputs must have same length".into());
    }
    
    let mut pos_dist = 0.0;
    let mut neg_dist = 0.0;
    
    for i in 0..anchor.len() {
        let ap_diff = anchor[i] - positive[i];
        let an_diff = anchor[i] - negative[i];
        pos_dist += ap_diff * ap_diff;
        neg_dist += an_diff * an_diff;
    }
    
    let loss = (pos_dist - neg_dist + margin).max(0.0);
    
    Ok(loss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_triplet_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let anchor = vec![1.0, 0.0, 0.0];
        let positive = vec![0.9, 0.1, 0.0];
        let negative = vec![0.0, 0.0, 1.0];
        let loss = triplet_loss(&dev.device, &dev.queue, &anchor, &positive, &negative, 0.2).await.unwrap();
        assert!(loss >= 0.0);
    }
}
