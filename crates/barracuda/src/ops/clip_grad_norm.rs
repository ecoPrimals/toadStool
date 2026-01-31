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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_clip_grad_norm_basic() {
        let dev = get_test_device().await;
        let grads = vec![3.0, 4.0]; // Norm = 5
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 1.0).await.unwrap();
        let norm: f32 = clipped.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_clip_grad_norm_edge_cases() {
        let dev = get_test_device().await;
        
        // Gradients already below max_norm (no clipping)
        let grads = vec![0.1, 0.2, 0.3]; // Norm ≈ 0.374
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 1.0).await.unwrap();
        assert_eq!(clipped, grads);
        
        // Zero gradients
        let grads = vec![0.0, 0.0, 0.0];
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 1.0).await.unwrap();
        assert_eq!(clipped, vec![0.0, 0.0, 0.0]);
    }

    #[tokio::test]
    async fn test_clip_grad_norm_boundary() {
        let dev = get_test_device().await;
        
        // Test with different max_norm values
        let grads = vec![6.0, 8.0]; // Norm = 10
        
        let clipped1 = clip_grad_norm(&dev.device, &dev.queue, &grads, 5.0).await.unwrap();
        let norm1: f32 = clipped1.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!((norm1 - 5.0).abs() < 1e-5);
        
        let clipped2 = clip_grad_norm(&dev.device, &dev.queue, &grads, 2.0).await.unwrap();
        let norm2: f32 = clipped2.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!((norm2 - 2.0).abs() < 1e-5);
        
        // Smaller max_norm should clip more
        assert!(clipped2.iter().map(|&x| x.abs()).sum::<f32>() < clipped1.iter().map(|&x| x.abs()).sum::<f32>());
    }

    #[tokio::test]
    async fn test_clip_grad_norm_large_batch() {
        let dev = get_test_device().await;
        
        // Large gradient vector
        let size = 1000;
        let grads: Vec<f32> = (0..size).map(|i| (i % 10) as f32).collect();
        
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 100.0).await.unwrap();
        
        assert_eq!(clipped.len(), size);
        assert!(clipped.iter().all(|&x| x.is_finite()));
        
        // Verify clipped norm
        let norm: f32 = clipped.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!(norm <= 100.0 + 0.01);
    }

    #[tokio::test]
    async fn test_clip_grad_norm_precision() {
        let dev = get_test_device().await;
        
        // Test proportional scaling
        let grads = vec![3.0, 4.0]; // Norm = 5
        let clipped = clip_grad_norm(&dev.device, &dev.queue, &grads, 2.5).await.unwrap();
        
        // Should scale by 2.5/5 = 0.5
        assert!((clipped[0] - 1.5).abs() < 1e-5);
        assert!((clipped[1] - 2.0).abs() < 1e-5);
        
        // Verify final norm
        let norm: f32 = clipped.iter().map(|&g| g * g).sum::<f32>().sqrt();
        assert!((norm - 2.5).abs() < 1e-5);
    }
}
