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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_smooth_l1_loss_basic() {
        let dev = get_test_device().await;
        let preds = vec![1.0, 2.0, 3.0];
        let targets = vec![1.5, 2.5, 3.5];
        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss >= 0.0 && loss.is_finite());
    }

    #[tokio::test]
    async fn test_smooth_l1_loss_edge_cases() {
        let dev = get_test_device().await;

        // Perfect predictions (loss = 0)
        let preds = vec![1.0, 2.0, 3.0];
        let targets = vec![1.0, 2.0, 3.0];
        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss.abs() < 1e-6);

        // Small differences (quadratic region)
        let preds = vec![1.0, 2.0];
        let targets = vec![1.1, 2.1];
        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss > 0.0 && loss < 0.1);
    }

    #[tokio::test]
    async fn test_smooth_l1_loss_boundary() {
        let dev = get_test_device().await;

        // Large differences (linear region)
        let preds = vec![0.0, 10.0];
        let targets = vec![5.0, 5.0];
        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss > 1.0);

        // Different beta values
        let preds = vec![1.0, 2.0];
        let targets = vec![2.0, 3.0];
        let loss_beta1 = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        let loss_beta2 = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 2.0)
            .await
            .unwrap();
        assert!(loss_beta1 != loss_beta2);
    }

    #[tokio::test]
    async fn test_smooth_l1_loss_large_batch() {
        let dev = get_test_device().await;

        let size = 1000;
        let preds: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let targets: Vec<f32> = (0..size).map(|i| (i + 1) as f32).collect();

        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, 1.0)
            .await
            .unwrap();
        assert!(loss >= 0.0 && loss.is_finite());
    }

    #[tokio::test]
    async fn test_smooth_l1_loss_precision() {
        let dev = get_test_device().await;

        // Test smooth transition at beta boundary
        let beta = 1.0;
        let preds = vec![0.0, 0.0, 0.0];
        let targets = vec![0.5, 1.0, 2.0]; // Below, at, and above beta

        let loss = smooth_l1_loss(&dev.device, &dev.queue, &preds, &targets, beta)
            .await
            .unwrap();
        assert!(loss > 0.0 && loss.is_finite());

        // Verify loss is smooth (no discontinuities)
        let loss_small = smooth_l1_loss(&dev.device, &dev.queue, &vec![0.0], &vec![0.99], beta)
            .await
            .unwrap();
        let loss_large = smooth_l1_loss(&dev.device, &dev.queue, &vec![0.0], &vec![1.01], beta)
            .await
            .unwrap();
        assert!((loss_small - loss_large).abs() < 0.5); // Smooth transition
    }
}
