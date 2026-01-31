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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_multi_margin_loss_basic() {
        let dev = get_test_device().await;
        let predictions = vec![0.9, 0.1, 0.1, 0.1, 0.8, 0.2];
        let targets = vec![0, 1];
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_multi_margin_loss_edge_cases() {
        let dev = get_test_device().await;

        // Perfect predictions (loss should be 0)
        let predictions = vec![10.0, 0.0, 0.0];
        let targets = vec![0];
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        assert!(loss < 0.1);

        // Single sample
        let predictions = vec![0.5, 0.5, 0.5];
        let targets = vec![0];
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        assert!(loss >= 0.0);
    }

    #[tokio::test]
    async fn test_multi_margin_loss_boundary() {
        let dev = get_test_device().await;

        // Different margins
        let predictions = vec![0.7, 0.3, 0.2];
        let targets = vec![0];
        let loss1 = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 0.5).await.unwrap();
        let loss2 = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 2.0).await.unwrap();
        assert!(loss2 > loss1); // Larger margin = larger loss
    }

    #[tokio::test]
    async fn test_multi_margin_loss_large_batch() {
        let dev = get_test_device().await;

        // Batch of 100
        let mut predictions = Vec::new();
        let mut targets = Vec::new();
        for i in 0..100 {
            predictions.extend(vec![0.8, 0.1, 0.1]);
            targets.push(i % 3);
        }
        
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_multi_margin_loss_precision() {
        let dev = get_test_device().await;

        // Test loss calculation
        let predictions = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let targets = vec![0, 1];
        let loss = multi_margin_loss(&dev.device, &dev.queue, &predictions, &targets, 3, 1.0).await.unwrap();
        
        // For perfect predictions, loss should be near 0
        assert!(loss < 0.5);
    }
}
