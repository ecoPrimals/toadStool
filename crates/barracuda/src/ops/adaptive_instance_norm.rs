//! Adaptive Instance Normalization (AdaIN) - Style transfer
//!
//! Transfers style from one image to another.
//! Used in neural style transfer, GANs.

pub async fn adaptive_instance_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    content: &[f32],
    style_mean: &[f32], // [channels]
    style_std: &[f32],  // [channels]
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; content.len()];

    for b in 0..batch_size {
        for c in 0..channels {
            // Compute content statistics
            let mut content_mean = 0.0;
            let mut content_var = 0.0;

            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                content_mean += content[idx];
            }
            content_mean /= spatial_size as f32;

            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let diff = content[idx] - content_mean;
                content_var += diff * diff;
            }
            content_var /= spatial_size as f32;
            let content_std = content_var.sqrt();

            // Apply AdaIN: normalize content, then scale/shift to style
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let normalized = (content[idx] - content_mean) / (content_std + 1e-5);
                output[idx] = normalized * style_std[c] + style_mean[c];
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
    async fn test_adaptive_instance_norm_basic() {
        let dev = get_test_device().await;
        let content = vec![1.0; 1 * 3 * 4 * 4];
        let style_mean = vec![0.5, 0.5, 0.5];
        let style_std = vec![0.2, 0.2, 0.2];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            3,
            4,
            4,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), content.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_edge_cases() {
        let dev = get_test_device().await;

        // Test with zero style std (should clamp)
        let content = vec![1.0, 2.0, 3.0, 4.0];
        let style_mean = vec![0.0];
        let style_std = vec![0.0];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Test with single channel, single pixel
        let content = vec![5.0];
        let style_mean = vec![1.0];
        let style_std = vec![2.0];
        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            1,
            1,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_boundary() {
        let dev = get_test_device().await;

        // Test with different style statistics
        let content = vec![0.0, 1.0, 2.0, 3.0];

        // Style 1: mean=0, std=1
        let output1 = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &vec![0.0],
            &vec![1.0],
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        // Style 2: mean=10, std=5
        let output2 = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &vec![10.0],
            &vec![5.0],
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        assert!(output1.iter().all(|&x| x.is_finite()));
        assert!(output2.iter().all(|&x| x.is_finite()));
        // Different style should produce different output
        assert_ne!(output1, output2);
        // Output2 should have higher values (mean=10)
        assert!(output2.iter().sum::<f32>() > output1.iter().sum::<f32>());
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_large_batch() {
        let dev = get_test_device().await;

        // Multiple batches and channels
        let batch_size = 2;
        let channels = 4;
        let height = 8;
        let width = 8;

        let content: Vec<f32> = (0..batch_size * channels * height * width)
            .map(|i| (i % 10) as f32)
            .collect();
        let style_mean = vec![0.5, 1.0, 1.5, 2.0];
        let style_std = vec![0.1, 0.2, 0.3, 0.4];

        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            batch_size,
            channels,
            height,
            width,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), content.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adaptive_instance_norm_precision() {
        let dev = get_test_device().await;

        // Test with known values for style transfer
        let content = vec![
            0.0, 1.0, 2.0, 3.0, // Mean = 1.5
        ];
        let style_mean = vec![5.0]; // Target mean
        let style_std = vec![2.0]; // Target std

        let output = adaptive_instance_norm(
            &dev.device,
            &dev.queue,
            &content,
            &style_mean,
            &style_std,
            1,
            1,
            2,
            2,
        )
        .await
        .unwrap();

        // After AdaIN, output should have approximately the target mean
        let out_mean = output.iter().sum::<f32>() / output.len() as f32;
        assert!((out_mean - 5.0).abs() < 0.1);

        // Output should preserve relative relationships (normalized)
        assert!(output[0] < output[1]);
        assert!(output[1] < output[2]);
        assert!(output[2] < output[3]);
    }
}
