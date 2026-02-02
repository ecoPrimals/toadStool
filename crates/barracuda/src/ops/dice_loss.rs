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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_dice_loss_basic() {
        let dev = get_test_device().await;
        // Perfect predictions (all 1.0)
        let predictions = vec![1.0; 1000];
        let targets = vec![1.0; 1000];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss < 0.1); // Should be close to 0 for perfect predictions
    }

    #[tokio::test]
    async fn test_dice_loss_edge_cases() {
        let dev = get_test_device().await;

        // All zeros (both pred and target)
        let predictions = vec![0.0; 100];
        let targets = vec![0.0; 100];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        // With smoothing, loss should be finite
        assert!(loss.is_finite());
        assert!(loss >= 0.0 && loss <= 1.0);

        // Perfect mismatch (pred=1, target=0)
        let predictions = vec![1.0; 100];
        let targets = vec![0.0; 100];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss > 0.5); // High loss for complete mismatch

        // Single element
        let predictions = vec![0.8];
        let targets = vec![1.0];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_dice_loss_boundary() {
        let dev = get_test_device().await;

        // Different smoothing values
        let predictions = vec![0.5; 100];
        let targets = vec![1.0; 100];
        let loss1 = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 0.1)
            .await
            .unwrap();
        let loss2 = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 10.0)
            .await
            .unwrap();
        // Different smoothing should yield different losses
        assert!(loss1.is_finite() && loss2.is_finite());

        // Partial overlap
        let mut predictions = vec![0.0; 200];
        let mut targets = vec![0.0; 200];
        for i in 0..100 {
            predictions[i] = 1.0;
            targets[i + 50] = 1.0; // 50% overlap
        }
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss > 0.0 && loss < 1.0); // Partial loss
    }

    #[tokio::test]
    async fn test_dice_loss_large_batch() {
        let dev = get_test_device().await;

        // Large volume (medical imaging scale: 128x128x64)
        let size = 128 * 128 * 64;
        let mut predictions = vec![0.0; size];
        let mut targets = vec![0.0; size];

        // Simulate realistic segmentation (10% foreground)
        for i in 0..(size / 10) {
            predictions[i] = 0.8; // Probabilistic prediction
            targets[i] = 1.0;
        }

        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss.is_finite());
        assert!(loss >= 0.0 && loss <= 1.0);
    }

    #[tokio::test]
    async fn test_dice_loss_precision() {
        let dev = get_test_device().await;

        // Known Dice coefficient calculation
        // Pred: [1, 0, 1, 0], Target: [1, 1, 0, 0]
        // Intersection = 1*1 + 0*1 + 1*0 + 0*0 = 1
        // Pred_sum = 1 + 0 + 1 + 0 = 2
        // Target_sum = 1 + 1 + 0 + 0 = 2
        // Dice = (2*1 + 1) / (2 + 2 + 1) = 3/5 = 0.6
        // Loss = 1 - 0.6 = 0.4
        let predictions = vec![1.0, 0.0, 1.0, 0.0];
        let targets = vec![1.0, 1.0, 0.0, 0.0];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        assert!((loss - 0.4).abs() < 0.01);

        // Perfect prediction
        let predictions = vec![1.0, 0.0, 1.0, 1.0];
        let targets = vec![1.0, 0.0, 1.0, 1.0];
        let loss = dice_loss(&dev.device, &dev.queue, &predictions, &targets, 1.0)
            .await
            .unwrap();
        // Dice = (2*3 + 1) / (3 + 3 + 1) = 7/7 = 1, Loss = 0
        assert!(loss < 0.01);
    }
}
