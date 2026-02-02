//! Focal Loss v2 - Enhanced focal loss with alpha balancing
//!
//! Improved version with class balancing parameter.

pub async fn focal_loss_v2(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32],
    targets: &[f32],
    alpha: f32,
    gamma: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }

    let mut total_loss = 0.0;
    const EPSILON: f32 = 1e-7;

    for i in 0..predictions.len() {
        let p = predictions[i].max(EPSILON).min(1.0 - EPSILON);
        let t = targets[i];

        let focal_weight = if t == 1.0 {
            alpha * (1.0 - p).powf(gamma)
        } else {
            (1.0 - alpha) * p.powf(gamma)
        };

        let bce = -(t * p.ln() + (1.0 - t) * (1.0 - p).ln());
        total_loss += focal_weight * bce;
    }

    Ok(total_loss / predictions.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_focal_loss_v2_basic() {
        let dev = get_test_device().await;
        let preds = vec![0.9, 0.1, 0.8];
        let targets = vec![1.0, 0.0, 1.0];
        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0)
            .await
            .unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_focal_loss_v2_edge_cases() {
        let dev = get_test_device().await;

        // Perfect predictions (loss should be near zero)
        let preds = vec![1.0, 0.0, 1.0, 0.0];
        let targets = vec![1.0, 0.0, 1.0, 0.0];
        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0)
            .await
            .unwrap();
        assert!(loss < 0.1, "Perfect predictions should have low loss");

        // All wrong predictions (high loss)
        let preds = vec![0.0, 1.0, 0.0, 1.0];
        let targets = vec![1.0, 0.0, 1.0, 0.0];
        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0)
            .await
            .unwrap();
        assert!(loss > 1.0, "Wrong predictions should have high loss");
    }

    #[tokio::test]
    async fn test_focal_loss_v2_boundary() {
        let dev = get_test_device().await;

        // Extreme but valid predictions (near 0 and 1)
        let preds = vec![0.999, 0.001, 0.95, 0.05];
        let targets = vec![1.0, 0.0, 1.0, 0.0];
        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0)
            .await
            .unwrap();
        assert!(loss >= 0.0 && loss.is_finite());

        // Test different gamma values
        let loss_gamma1 = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 1.0)
            .await
            .unwrap();
        let loss_gamma3 = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 3.0)
            .await
            .unwrap();
        assert!(loss_gamma1 != loss_gamma3);
    }

    #[tokio::test]
    async fn test_focal_loss_v2_large_batch() {
        let dev = get_test_device().await;

        // Large batch size
        let size = 1000;
        let preds: Vec<f32> = (0..size).map(|i| (i % 100) as f32 / 100.0).collect();
        let targets: Vec<f32> = (0..size)
            .map(|i| if i % 2 == 0 { 1.0 } else { 0.0 })
            .collect();

        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0)
            .await
            .unwrap();
        assert!(loss >= 0.0 && loss.is_finite());
    }

    #[tokio::test]
    async fn test_focal_loss_v2_precision() {
        let dev = get_test_device().await;

        // Test alpha balancing effect
        let preds = vec![0.7, 0.3, 0.6, 0.4];
        let targets = vec![1.0, 0.0, 1.0, 0.0];

        let loss_alpha_low = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.1, 2.0)
            .await
            .unwrap();
        let loss_alpha_high = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.9, 2.0)
            .await
            .unwrap();

        // Alpha should affect the loss magnitude
        assert!(loss_alpha_low != loss_alpha_high);
        assert!(loss_alpha_low.is_finite() && loss_alpha_high.is_finite());
    }
}
