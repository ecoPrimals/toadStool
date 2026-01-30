//! Multi-Margin Loss - Multi-class hinge loss
//!
//! SVM-style loss for multi-class classification.

pub async fn multi_margin_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32], // [batch, num_classes]
    targets: &[usize],   // [batch] (class indices)
    num_classes: usize,
    margin: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let batch_size = targets.len();
    if predictions.len() != batch_size * num_classes {
        return Err("Dimension mismatch".into());
    }
    
    let mut total_loss = 0.0;
    
    for i in 0..batch_size {
        let target_class = targets[i];
        let target_score = predictions[i * num_classes + target_class];
        
        let mut class_loss = 0.0;
        for c in 0..num_classes {
            if c != target_class {
                let score = predictions[i * num_classes + c];
                class_loss += (margin - target_score + score).max(0.0);
            }
        }
        
        total_loss += class_loss / (num_classes - 1) as f32;
    }
    
    Ok(total_loss / batch_size as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_multi_margin_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let predictions = vec![0.9, 0.1, 0.1, 0.1, 0.8, 0.2];
        let targets = vec![0, 1];
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        assert!(loss >= 0.0);
    }
}
