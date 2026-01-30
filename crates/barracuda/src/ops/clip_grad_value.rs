//! Clip Gradient by Value - Elementwise gradient clipping
//!
//! Clips each gradient value to [-max, max].

pub async fn clip_grad_value(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    gradients: &[f32],
    clip_value: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let clipped: Vec<f32> = gradients.iter()
        .map(|&g| g.max(-clip_value).min(clip_value))
        .collect();
    
    Ok(clipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_clip_grad_value() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let grads = vec![-5.0, -1.0, 0.0, 1.0, 5.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 2.0).await.unwrap();
        assert_eq!(clipped, vec![-2.0, -1.0, 0.0, 1.0, 2.0]);
    }
}
