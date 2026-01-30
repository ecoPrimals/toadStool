//! DiceLoss - Dice loss for segmentation
//!
//! Optimizes IoU-like metric directly.
//! Popular for medical image segmentation.

pub async fn dice_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32], // Probabilities [0, 1]
    targets: &[f32],     // Binary targets {0, 1}
    smooth: f32,         // Smoothing factor to avoid division by zero
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }
    
    let mut intersection = 0.0;
    let mut pred_sum = 0.0;
    let mut target_sum = 0.0;
    
    for i in 0..predictions.len() {
        intersection += predictions[i] * targets[i];
        pred_sum += predictions[i];
        target_sum += targets[i];
    }
    
    // Dice coefficient: 2 * |A ∩ B| / (|A| + |B|)
    let dice = (2.0 * intersection + smooth) / (pred_sum + target_sum + smooth);
    
    // Dice loss = 1 - Dice coefficient
    Ok(1.0 - dice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_dice_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let predictions = vec![1.0; 1000];
        let targets = vec![1.0; 1000];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0).await.unwrap();
        assert!(loss < 0.1); // Should be close to 0 for perfect predictions
    }
}
