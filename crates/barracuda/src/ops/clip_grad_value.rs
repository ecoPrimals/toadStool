//! Clip Gradient by Value - Elementwise gradient clipping
//!
//! Clips each gradient value to [-max, max].

pub async fn clip_grad_value(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    gradients: &[f32],
    clip_value: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let clipped: Vec<f32> = gradients
        .iter()
        .map(|&g| g.max(-clip_value).min(clip_value))
        .collect();

    Ok(clipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_clip_grad_value_basic() {
        let dev = get_test_device().await;
        let grads = vec![-5.0, -1.0, 0.0, 1.0, 5.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 2.0)
            .await
            .unwrap();
        assert_eq!(clipped, vec![-2.0, -1.0, 0.0, 1.0, 2.0]);
    }

    #[tokio::test]
    async fn test_clip_grad_value_edge_cases() {
        let dev = get_test_device().await;

        // All values within clip range (no clipping)
        let grads = vec![0.5, -0.5, 1.0, -1.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 2.0)
            .await
            .unwrap();
        assert_eq!(clipped, grads);

        // Zero gradients
        let grads = vec![0.0, 0.0, 0.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 1.0)
            .await
            .unwrap();
        assert_eq!(clipped, vec![0.0, 0.0, 0.0]);
    }

    #[tokio::test]
    async fn test_clip_grad_value_boundary() {
        let dev = get_test_device().await;

        // Values exactly at clip boundary
        let grads = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 2.0)
            .await
            .unwrap();
        assert_eq!(clipped, vec![-2.0, -1.0, 0.0, 1.0, 2.0]);

        // Very large values
        let grads = vec![-100.0, 0.0, 100.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 10.0)
            .await
            .unwrap();
        assert_eq!(clipped, vec![-10.0, 0.0, 10.0]);
    }

    #[tokio::test]
    async fn test_clip_grad_value_large_batch() {
        let dev = get_test_device().await;

        // Large gradient vector
        let size = 1000;
        let grads: Vec<f32> = (0..size).map(|i| (i as f32 - 500.0) * 0.1).collect();

        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 10.0)
            .await
            .unwrap();

        assert_eq!(clipped.len(), size);
        assert!(clipped.iter().all(|&x| x >= -10.0 && x <= 10.0));
    }

    #[tokio::test]
    async fn test_clip_grad_value_precision() {
        let dev = get_test_device().await;

        // Test symmetric clipping
        let grads = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
        let clipped = clip_grad_value(&dev.device, &dev.queue, &grads, 5.0)
            .await
            .unwrap();

        assert_eq!(clipped[0], -5.0);
        assert_eq!(clipped[1], -5.0);
        assert_eq!(clipped[2], 0.0);
        assert_eq!(clipped[3], 5.0);
        assert_eq!(clipped[4], 5.0);

        // All values should be within [-5, 5]
        assert!(clipped.iter().all(|&x| x >= -5.0 && x <= 5.0));
    }
}
