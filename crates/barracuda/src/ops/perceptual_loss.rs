//! PerceptualLoss - Feature-based perceptual loss
//!
//! Compares high-level features instead of pixels.
//! Used in style transfer and super-resolution.

pub async fn perceptual_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    features1: &[f32], // Features from layer [N, C, H, W] flattened
    features2: &[f32],
    weights: Option<&[f32]>, // Optional per-layer weights
) -> Result<f32, Box<dyn std::error::Error>> {
    if features1.len() != features2.len() {
        return Err("Feature dimensions must match".into());
    }
    
    let mut loss = 0.0;
    
    if let Some(w) = weights {
        // Weighted feature comparison
        if w.len() * features1.len() / w.len() != features1.len() {
            return Err("Weights dimension mismatch".into());
        }
        
        let features_per_weight = features1.len() / w.len();
        
        for i in 0..w.len() {
            let start = i * features_per_weight;
            let end = start + features_per_weight;
            
            for j in start..end {
                let diff = features1[j] - features2[j];
                loss += w[i] * diff * diff;
            }
        }
    } else {
        // Unweighted MSE on features
        for i in 0..features1.len() {
            let diff = features1[i] - features2[i];
            loss += diff * diff;
        }
    }
    
    loss /= features1.len() as f32;
    
    Ok(loss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_perceptual_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let features1 = vec![0.5; 1000];
        let features2 = vec![0.6; 1000];
        let loss = perceptual_loss(&dev.device, &dev.queue, &features1, &features2, None).await.unwrap();
        assert!(loss > 0.0);
    }
}
