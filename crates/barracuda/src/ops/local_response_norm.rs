//! Local Response Normalization (LRN) - AlexNet-style normalization
//!
//! Normalizes across nearby channels.
//! Used in AlexNet (historic).

pub async fn local_response_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    size: usize, // Normalization window size
    alpha: f32,
    beta: f32,
    k: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; input.len()];
    let half_size = size / 2;

    for b in 0..batch_size {
        for c in 0..channels {
            let c_start = c.saturating_sub(half_size);
            let c_end = (c + half_size + 1).min(channels);

            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let x = input[idx];

                // Sum of squares in local window
                let mut sum_sq = 0.0;
                for nc in c_start..c_end {
                    let n_idx = b * channels * spatial_size + nc * spatial_size + s;
                    sum_sq += input[n_idx] * input[n_idx];
                }

                let denom = (k + alpha * sum_sq).powf(beta);
                output[idx] = x / denom;
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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_local_response_norm_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 4 * 4 * 4];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            4,
            4,
            4,
            3,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), input.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_local_response_norm_edge_cases() {
        let dev = get_test_device().await;

        // Single channel (no cross-channel normalization)
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            1,
            4,
            4,
            3,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 16);

        // Small window size (1)
        let input = vec![1.0; 1 * 4 * 4 * 4];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            4,
            4,
            4,
            1,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_local_response_norm_boundary() {
        let dev = get_test_device().await;

        // Large window (covers all channels)
        let input = vec![1.0; 1 * 4 * 4 * 4];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            4,
            4,
            4,
            5,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Different alpha/beta/k values
        let input = vec![1.0; 1 * 8 * 4 * 4];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            8,
            4,
            4,
            5,
            0.001,
            0.5,
            2.0,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), input.len());
    }

    #[tokio::test]
    async fn test_local_response_norm_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4, AlexNet style
        let batch_size = 4;
        let channels = 8;
        let input = vec![1.0; batch_size * channels * 8 * 8];
        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            batch_size,
            channels,
            8,
            8,
            5,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), input.len());
    }

    #[tokio::test]
    async fn test_local_response_norm_precision() {
        let dev = get_test_device().await;

        // Test normalization with known values
        let mut input = vec![0.0; 1 * 3 * 2 * 2];
        input[0..4].fill(1.0); // Channel 0
        input[4..8].fill(2.0); // Channel 1
        input[8..12].fill(3.0); // Channel 2

        let output = local_response_norm(
            &dev.device,
            &dev.queue,
            &input,
            1,
            3,
            2,
            2,
            3,
            0.0001,
            0.75,
            1.0,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), 12);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Normalized values should be less than original (divisive normalization)
        assert!(output[4] <= 2.0); // Channel 1 should be normalized down
    }
}
