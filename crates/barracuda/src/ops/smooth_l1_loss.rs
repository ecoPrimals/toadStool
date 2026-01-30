//! Smooth L1 Loss - Less sensitive to outliers than L2
//!
//! Combines L1 and L2 loss properties.
//! Used in object detection (Faster R-CNN).

pub async fn smooth_l1_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32],
    targets: &[f32],
    beta: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }
    
    let mut total_loss = 0.0;
    
    for i in 0..predictions.len() {
        let diff = (predictions[i] - targets[i]).abs();
        
        let loss = if diff < beta {
            0.5 * diff * diff / beta
        } else {
            diff - 0.5 * beta
        };
        
        total_loss += loss;
    }
    
    Ok(total_loss / predictions.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_smooth_l1_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let preds = vec![1.0, 2.0, 3.0];
        let targets = vec![1.5, 2.5, 3.5];
        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0).await.unwrap();
        assert!(loss >= 0.0);
    }
}
