//! LayerScale - Per-layer learnable scaling
//!
//! Used in vision transformers (CaiT, LeViT) to stabilize training.
//!
//! ## Algorithm
//!
//! ```text
//! LayerScale(x) = gamma ⊙ x
//! ```
//!
//! Where gamma is a learnable per-channel parameter.

pub async fn layer_scale(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    gamma: &[f32], // Per-channel scaling factors
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if gamma.len() != input.len() {
        return Err("Gamma must match input length".into());
    }

    let output: Vec<f32> = input
        .iter()
        .zip(gamma.iter())
        .map(|(&x, &g)| x * g)
        .collect();

    Ok(output)
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
    async fn test_layer_scale_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 3.0];
        let gamma = vec![0.1, 0.2, 0.3];
        let output = layer_scale(&device, &queue, &input, &gamma).await.unwrap();
        assert_eq!(output.len(), 3);
        assert!((output[0] - 0.1).abs() < 1e-5);
        assert!((output[1] - 0.4).abs() < 1e-5);
        assert!((output[2] - 0.9).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_layer_scale_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let input = vec![5.0];
        let gamma = vec![0.5];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();
        assert_eq!(output.len(), 1);
        assert!((output[0] - 2.5).abs() < 1e-5);

        // All zeros
        let input = vec![0.0, 0.0, 0.0];
        let gamma = vec![1.0, 2.0, 3.0];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();
        assert!(output.iter().all(|&x| x.abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_layer_scale_boundary() {
        let dev = get_test_device().await;

        // Gamma = 0 (complete suppression)
        let input = vec![1.0, 2.0, 3.0];
        let gamma = vec![0.0, 0.0, 0.0];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();
        assert!(output.iter().all(|&x| x.abs() < 1e-5));

        // Gamma = 1 (identity)
        let input = vec![1.0, 2.0, 3.0];
        let gamma = vec![1.0, 1.0, 1.0];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_layer_scale_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let gamma: Vec<f32> = vec![0.5; 1000];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();

        assert_eq!(output.len(), 1000);
        // All should be halved
        for i in 0..1000 {
            assert!((output[i] - input[i] * 0.5).abs() < 1e-3);
        }
    }

    #[tokio::test]
    async fn test_layer_scale_precision() {
        let dev = get_test_device().await;

        // Test with various gamma values
        let input = vec![10.0, 20.0, 30.0];
        let gamma = vec![0.01, 0.1, 1.0];
        let output = layer_scale(&dev.device, &dev.queue, &input, &gamma)
            .await
            .unwrap();

        assert!((output[0] - 0.1).abs() < 1e-5); // 10 * 0.01
        assert!((output[1] - 2.0).abs() < 1e-5); // 20 * 0.1
        assert!((output[2] - 30.0).abs() < 1e-5); // 30 * 1.0
    }
}
