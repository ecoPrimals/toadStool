//! TverskyLoss - Tversky loss (generalized Dice)
//!
//! Asymmetric similarity measure with control over FP/FN.
//! Useful for imbalanced segmentation tasks.

pub async fn tversky_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predictions: &[f32],
    targets: &[f32],
    alpha: f32, // Weight for false positives
    beta: f32,  // Weight for false negatives
    smooth: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if predictions.len() != targets.len() {
        return Err("Predictions and targets must have same length".into());
    }

    let mut true_pos = 0.0;
    let mut false_pos = 0.0;
    let mut false_neg = 0.0;

    for i in 0..predictions.len() {
        let pred = predictions[i];
        let target = targets[i];

        true_pos += pred * target;
        false_pos += pred * (1.0 - target);
        false_neg += (1.0 - pred) * target;
    }

    // Tversky index
    let tversky = (true_pos + smooth) / (true_pos + alpha * false_pos + beta * false_neg + smooth);

    // Tversky loss = 1 - Tversky index
    Ok(1.0 - tversky)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tversky_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let predictions = vec![0.9; 800];
        let targets = vec![1.0; 800];
        let loss = tversky_loss(
            &dev.device,
            &dev.queue,
            &predictions,
            &targets,
            0.5,
            0.5,
            1.0,
        )
        .await
        .unwrap();
        assert!(loss >= 0.0 && loss <= 1.0);
    }
}
