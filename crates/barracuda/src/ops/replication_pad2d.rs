//! ReplicationPad2D - Padding by replicating edge pixels
//!
//! Pads by repeating border values.
//! Common in image processing.

pub async fn replication_pad2d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    pad_top: usize,
    pad_bottom: usize,
    pad_left: usize,
    pad_right: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = height + pad_top + pad_bottom;
    let out_w = width + pad_left + pad_right;
    let mut output = vec![0.0f32; batch_size * channels * out_h * out_w];

    for b in 0..batch_size {
        for c in 0..channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let ih = (oh as isize - pad_top as isize)
                        .max(0)
                        .min(height as isize - 1) as usize;
                    let iw = (ow as isize - pad_left as isize)
                        .max(0)
                        .min(width as isize - 1) as usize;

                    let in_idx =
                        b * channels * height * width + c * height * width + ih * width + iw;
                    let out_idx =
                        b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
                    output[out_idx] = input[in_idx];
                }
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
    async fn test_replication_pad2d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 1, 1, 1)
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);
    }

    #[tokio::test]
    async fn test_replication_pad2d_edge_cases() {
        let dev = get_test_device().await;

        // No padding
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 0, 0, 0, 0)
            .await
            .unwrap();
        assert_eq!(output, input);

        // Single pixel
        let input = vec![5.0];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 1, 1, 2, 2, 2, 2)
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 1 * 5 * 5);
        assert!(output.iter().all(|&x| x == 5.0)); // All replicated to 5.0
    }

    #[tokio::test]
    async fn test_replication_pad2d_boundary() {
        let dev = get_test_device().await;

        // Large padding
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 3, 4, 4, 3, 3, 3, 3)
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 3 * 10 * 10);

        // Asymmetric padding
        let input = vec![1.0; 1 * 1 * 8 * 8];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 8, 8, 2, 3, 1, 4)
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 1 * 13 * 13);
    }

    #[tokio::test]
    async fn test_replication_pad2d_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4, multiple channels
        let batch_size = 4;
        let channels = 3;
        let input = vec![1.0; batch_size * channels * 16 * 16];
        let output = replication_pad2d(
            &dev.device,
            &dev.queue,
            &input,
            batch_size,
            channels,
            16,
            16,
            2,
            2,
            2,
            2,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), batch_size * channels * 20 * 20);
    }

    #[tokio::test]
    async fn test_replication_pad2d_precision() {
        let dev = get_test_device().await;

        // Verify replication behavior
        let input = vec![1.0, 2.0, 3.0, 4.0]; // [[1,2],[3,4]]
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 1, 1, 1)
            .await
            .unwrap();

        assert_eq!(output.len(), 16); // 4x4
        assert!(output.iter().all(|&x| x.is_finite()));
        // All values should be from input (1, 2, 3, 4)
        assert!(output.iter().all(|&x| x >= 1.0 && x <= 4.0));
    }
}
