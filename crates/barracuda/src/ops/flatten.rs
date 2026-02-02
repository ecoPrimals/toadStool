//! Flatten - Flatten tensor to 1D or 2D
//!
//! Collapses dimensions.

pub async fn flatten(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    _start_dim: usize,
    _end_dim: usize,
    _shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simple case: just return copy (reshape is metadata operation)
    Ok(input.to_vec())
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
    async fn test_flatten_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 2 * 3 * 4];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 2, &[2, 3, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), input.len());
        assert_eq!(output.len(), 24);
    }

    #[tokio::test]
    async fn test_flatten_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let input = vec![42.0];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 0, &[1])
            .await
            .unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], 42.0);

        // Already 1D
        let input = vec![1.0, 2.0, 3.0];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 0, &[3])
            .await
            .unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_flatten_boundary() {
        let dev = get_test_device().await;

        // 4D tensor
        let input = vec![1.0; 2 * 3 * 4 * 5];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 3, &[2, 3, 4, 5])
            .await
            .unwrap();
        assert_eq!(output.len(), 120);

        // Different values preserved
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let output = flatten(&dev.device, &dev.queue, &input, 0, 2, &[2, 2, 3])
            .await
            .unwrap();
        assert_eq!(output, input); // Values preserved
    }

    #[tokio::test]
    async fn test_flatten_large_batch() {
        let dev = get_test_device().await;

        // Large tensor
        let input = vec![1.0; 10 * 20 * 30];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 2, &[10, 20, 30])
            .await
            .unwrap();
        assert_eq!(output.len(), 6000);
        assert!(output.iter().all(|&x| x == 1.0));
    }

    #[tokio::test]
    async fn test_flatten_precision() {
        let dev = get_test_device().await;

        // Verify exact values preserved during flatten
        let input = vec![1.1, 2.2, 3.3, 4.4, 5.5, 6.6, 7.7, 8.8];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 1, &[2, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 8);
        for (i, val) in output.iter().enumerate() {
            assert!((val - input[i]).abs() < 1e-6);
        }
    }
}
