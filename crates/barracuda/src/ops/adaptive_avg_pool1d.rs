//! AdaptiveAvgPool1D - 1D adaptive average pooling
//!
//! Pools to fixed output size regardless of input size.

pub async fn adaptive_avg_pool1d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    length: usize,
    output_length: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * channels * output_length];

    for b in 0..batch_size {
        for c in 0..channels {
            for ol in 0..output_length {
                let start = (ol * length) / output_length;
                let end = ((ol + 1) * length) / output_length;

                let mut sum = 0.0;
                let count = end - start;

                for l in start..end {
                    let idx = b * channels * length + c * length + l;
                    sum += input[idx];
                }

                let out_idx = b * channels * output_length + c * output_length + ol;
                output[out_idx] = sum / count as f32;
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adaptive_avg_pool1d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 3 * 16];
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 3, 16, 8)
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 3 * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Constant input should produce constant output
        assert!(output.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_adaptive_avg_pool1d_edge_cases() {
        let dev = get_test_device().await;

        // Pool to output_length = 1 (global pooling)
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 1, 4, 1)
            .await
            .unwrap();
        assert_eq!(output.len(), 1);
        // Should be average of all: (1+2+3+4)/4 = 2.5
        assert!((output[0] - 2.5).abs() < 1e-6);

        // Pool to same size (no-op)
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 1, 4, 4)
            .await
            .unwrap();
        assert_eq!(output.len(), 4);
        assert!((output[0] - 1.0).abs() < 1e-6);
        assert!((output[1] - 2.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_adaptive_avg_pool1d_boundary() {
        let dev = get_test_device().await;

        // Downsampling (large to small)
        let input: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 1, 32, 4)
            .await
            .unwrap();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Each output should be average of 8 inputs
        assert!(output[0] < output[1]);
        assert!(output[1] < output[2]);

        // Different stride patterns
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 1, 8, 4)
            .await
            .unwrap();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adaptive_avg_pool1d_large_batch() {
        let dev = get_test_device().await;

        // Multiple batches and channels
        let batch_size = 4;
        let channels = 8;
        let length = 64;
        let output_length = 16;

        let input: Vec<f32> = (0..batch_size * channels * length)
            .map(|i| (i % 10) as f32)
            .collect();
        let output = adaptive_avg_pool1d(
            &dev.device,
            &dev.queue,
            &input,
            batch_size,
            channels,
            length,
            output_length,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), batch_size * channels * output_length);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Each batch/channel should be processed independently
        assert!(output.iter().all(|&x| x >= 0.0 && x < 10.0));
    }

    #[tokio::test]
    async fn test_adaptive_avg_pool1d_precision() {
        let dev = get_test_device().await;

        // Test with known values
        let input = vec![
            1.0, 2.0, 3.0, 4.0, // Channel 0
            5.0, 6.0, 7.0, 8.0, // Channel 1
        ];
        let output = adaptive_avg_pool1d(&dev.device, &dev.queue, &input, 1, 2, 4, 2)
            .await
            .unwrap();

        // Channel 0: pool [1,2] and [3,4]
        assert!((output[0] - 1.5).abs() < 1e-6); // (1+2)/2 = 1.5
        assert!((output[1] - 3.5).abs() < 1e-6); // (3+4)/2 = 3.5

        // Channel 1: pool [5,6] and [7,8]
        assert!((output[2] - 5.5).abs() < 1e-6); // (5+6)/2 = 5.5
        assert!((output[3] - 7.5).abs() < 1e-6); // (7+8)/2 = 7.5
    }
}
