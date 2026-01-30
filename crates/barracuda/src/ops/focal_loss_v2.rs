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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_focal_loss_v2() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let preds = vec![0.9, 0.1, 0.8];
        let targets = vec![1.0, 0.0, 1.0];
        let loss = focal_loss_v2(&dev.device, &dev.queue, &preds, &targets, 0.25, 2.0).await.unwrap();
        assert!(loss >= 0.0);
    }
}
