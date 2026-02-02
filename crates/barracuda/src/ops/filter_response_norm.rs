//! Filter Response Normalization (FRN) - Normalization without batch dependency
//!
//! Normalizes activations per filter, not per batch.
//! Enables single-sample inference.

pub async fn filter_response_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    gamma: &[f32], // [channels]
    beta: &[f32],  // [channels]
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; input.len()];

    for b in 0..batch_size {
        for c in 0..channels {
            // Compute squared norm for this filter
            let mut sum_sq = 0.0;
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                sum_sq += input[idx] * input[idx];
            }
            let nu = (sum_sq / spatial_size as f32).sqrt();

            // Normalize
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let normalized = input[idx] / (nu + epsilon);
                output[idx] = gamma[c] * normalized + beta[c];
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_filter_response_norm_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let gamma = vec![1.0; 3];
        let beta = vec![0.0; 3];
        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            1,
            3,
            4,
            4,
            1e-5,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), input.len());

        // All values should be normalized
        for &val in &output {
            assert!(val.is_finite());
        }
    }

    #[tokio::test]
    async fn test_filter_response_norm_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Zero input
        let zeros = vec![0.0; 1 * 2 * 3 * 3];
        let gamma = vec![1.0; 2];
        let beta = vec![0.0; 2];
        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &zeros,
            &gamma,
            &beta,
            1,
            2,
            3,
            3,
            1e-5,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), zeros.len());

        // With beta shift
        let input = vec![1.0; 1 * 2 * 4 * 4];
        let gamma = vec![1.0; 2];
        let beta = vec![0.5; 2];
        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            1,
            2,
            4,
            4,
            1e-5,
        )
        .await
        .unwrap();

        // Output should have beta shift applied
        for &val in &output {
            assert!(val > 0.0); // Should be positive due to beta
        }
    }

    #[tokio::test]
    async fn test_filter_response_norm_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Single channel
        let input = vec![2.0; 1 * 1 * 8 * 8];
        let gamma = vec![1.5];
        let beta = vec![0.1];
        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            1,
            1,
            8,
            8,
            1e-5,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 64);

        // Multiple batches
        let input = vec![1.0; 2 * 3 * 4 * 4];
        let gamma = vec![1.0; 3];
        let beta = vec![0.0; 3];
        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            2,
            3,
            4,
            4,
            1e-5,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 2 * 3 * 4 * 4);
    }

    #[tokio::test]
    async fn test_filter_response_norm_large_tensor() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Realistic CNN feature map (batch=2, channels=64, 32x32)
        let batch = 2;
        let channels = 64;
        let size = 32;
        let input_len = batch * channels * size * size;

        let input = vec![1.0; input_len];
        let gamma = vec![1.0; channels];
        let beta = vec![0.0; channels];

        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            batch,
            channels,
            size,
            size,
            1e-5,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), input_len);

        // Check normalization worked
        for &val in &output {
            assert!(val.is_finite());
        }
    }

    #[tokio::test]
    async fn test_filter_response_norm_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test with varying values per channel
        let mut input = vec![0.0; 1 * 2 * 4 * 4];

        // Channel 0: values 1-16
        for i in 0..16 {
            input[i] = (i + 1) as f32;
        }

        // Channel 1: all 2.0
        for i in 16..32 {
            input[i] = 2.0;
        }

        let gamma = vec![1.0, 2.0]; // Different scales
        let beta = vec![0.0, 0.5]; // Different shifts

        let output = filter_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            &gamma,
            &beta,
            1,
            2,
            4,
            4,
            1e-5,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), 32);

        // Channel 0 should be normalized with gamma=1.0, beta=0.0
        // Channel 1 should be normalized with gamma=2.0, beta=0.5
        for i in 16..32 {
            assert!(output[i] > 1.0); // Should be > 1 due to gamma=2.0 and beta=0.5
        }
    }
}
