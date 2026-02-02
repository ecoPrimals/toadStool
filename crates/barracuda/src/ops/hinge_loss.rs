//! Hinge Loss - SVM-style classification loss
//!
//! ## Algorithm
//!
//! ```text
//! HingeLoss = max(0, 1 - y * pred)
//! ```
//!
//! Where y ∈ {-1, +1} is true label, pred is prediction.

pub async fn hinge_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32],
    targets: &[f32], // Should be -1 or +1
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }

    let mut total_loss = 0.0;

    for i in 0..predictions.len() {
        let loss = (1.0 - targets[i] * predictions[i]).max(0.0);
        total_loss += loss;
    }

    Ok(total_loss / predictions.len() as f32)
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
    async fn test_hinge_loss_basic() {
        let dev = get_test_device().await;
        let predictions = vec![0.8, -0.5, 0.3];
        let targets = vec![1.0, -1.0, 1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_hinge_loss_edge_cases() {
        let dev = get_test_device().await;

        // Perfect predictions (loss = 0)
        let predictions = vec![2.0, -2.0, 1.5];
        let targets = vec![1.0, -1.0, 1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!(loss.abs() < 1e-5);

        // Single element
        let predictions = vec![0.5];
        let targets = vec![1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!(loss >= 0.0);
        // Loss = max(0, 1 - 1.0 * 0.5) = 0.5
        assert!((loss - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_hinge_loss_boundary() {
        let dev = get_test_device().await;

        // All wrong predictions
        let predictions = vec![-1.0, 1.0, -0.5];
        let targets = vec![1.0, -1.0, 1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!(loss > 0.0);

        // Mixed predictions
        let predictions = vec![0.0, 0.0, 0.0];
        let targets = vec![1.0, -1.0, 1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        // Loss = max(0, 1 - target * 0) = 1.0 for all
        assert!((loss - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_hinge_loss_large_batch() {
        let dev = get_test_device().await;

        // 1000 predictions
        let predictions: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 0.8 } else { -0.8 })
            .collect();
        let targets: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_hinge_loss_precision() {
        let dev = get_test_device().await;

        // Known hinge loss calculation
        // pred=0.5, target=1.0 → loss = max(0, 1 - 1.0*0.5) = 0.5
        let predictions = vec![0.5];
        let targets = vec![1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!((loss - 0.5).abs() < 0.01);

        // pred=-0.3, target=-1.0 → loss = max(0, 1 - (-1)*(-0.3)) = max(0, 1 - 0.3) = 0.7
        let predictions = vec![-0.3];
        let targets = vec![-1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets)
            .await
            .unwrap();
        assert!((loss - 0.7).abs() < 0.01);
    }
}
