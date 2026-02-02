//! Softsign activation
//!
//! ## Algorithm
//!
//! ```text
//! Softsign(x) = x / (1 + |x|)
//! ```
//!
//! Smooth alternative to tanh, bounded between -1 and 1.

pub async fn softsign(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = input.iter().map(|&x| x / (1.0 + x.abs())).collect();
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
    async fn test_softsign_basic() {
        let dev = get_test_device().await;
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 5);
        assert!((output[2] - 0.0).abs() < 1e-6); // 0 -> 0
        assert!(output.iter().all(|&x| x >= -1.0 && x <= 1.0)); // Bounded
    }

    #[tokio::test]
    async fn test_softsign_edge_cases() {
        let dev = get_test_device().await;

        // Single value
        let input = vec![0.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!((output[0] - 0.0).abs() < 1e-6);

        // Large positive
        let input = vec![100.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!(output[0] > 0.99); // Approaches 1

        // Large negative
        let input = vec![-100.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!(output[0] < -0.99); // Approaches -1
    }

    #[tokio::test]
    async fn test_softsign_boundary() {
        let dev = get_test_device().await;

        // Test bounds: softsign(x) = x / (1 + |x|) ∈ (-1, 1)
        let input = vec![-10.0, -1.0, 0.0, 1.0, 10.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();

        for &val in &output {
            assert!(val > -1.0 && val < 1.0);
        }
    }

    #[tokio::test]
    async fn test_softsign_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) / 10.0).collect();
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 1000);
        assert!(output.iter().all(|&x| x >= -1.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_softsign_precision() {
        let dev = get_test_device().await;

        // Known values: softsign(1) = 1/2 = 0.5
        let input = vec![1.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!((output[0] - 0.5).abs() < 1e-6);

        // softsign(-1) = -1/2 = -0.5
        let input = vec![-1.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!((output[0] - (-0.5)).abs() < 1e-6);

        // softsign(2) = 2/3
        let input = vec![2.0];
        let output = softsign(&dev.device, &dev.queue, &input).await.unwrap();
        assert!((output[0] - (2.0 / 3.0)).abs() < 1e-6);
    }
}
