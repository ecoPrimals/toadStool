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
    
    #[tokio::test]
    async fn test_hinge_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let predictions = vec![0.8, -0.5, 0.3];
        let targets = vec![1.0, -1.0, 1.0];
        let loss = hinge_loss(&dev.device, &dev.queue, &predictions, &targets).await.unwrap();
        assert!(loss >= 0.0);
    }
}
