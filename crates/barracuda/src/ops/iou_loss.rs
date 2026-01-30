//! IoULoss - Intersection over Union loss
//!
//! Direct optimization of IoU metric.
//! Used in segmentation and object detection.

pub async fn iou_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32],
    targets: &[f32],
    smooth: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }
    
    let mut intersection = 0.0;
    let mut union = 0.0;
    
    for i in 0..predictions.len() {
        intersection += predictions[i] * targets[i];
        union += predictions[i] + targets[i] - predictions[i] * targets[i];
    }
    
    // IoU = |A ∩ B| / |A ∪ B|
    let iou = (intersection + smooth) / (union + smooth);
    
    // IoU loss = 1 - IoU
    Ok(1.0 - iou)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_iou_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let predictions = vec![0.8; 500];
        let targets = vec![1.0; 500];
        let loss = iou_loss(&dev.device, &dev.queue, &predictions, &targets, 1e-6).await.unwrap();
        assert!(loss > 0.0 && loss < 1.0);
    }
}
