//! Clip Gradient by Norm - Prevent exploding gradients
//!
//! Clips gradient norm to maximum value.

pub async fn clip_grad_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    gradients: &[f32],
    max_norm: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut norm_sq = 0.0;
    for &g in gradients {
        norm_sq += g * g;
    }
    let norm = norm_sq.sqrt();
    
    if norm > max_norm {
        let scale = max_norm / norm;
        let clipped: Vec<f32> = gradients.iter().map(|&g| g * scale).collect();
        Ok(clipped)
    } else {
        Ok(gradients.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_clip_grad_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let grads = vec![3.0, 4.0]; // Norm = 5
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 1.0).await.unwrap();
        let norm: f32 = clipped.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
