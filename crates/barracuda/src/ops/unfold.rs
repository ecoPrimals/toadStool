//! Unfold - Extract sliding local blocks (im2col)
//!
//! Extracts sliding windows as columns.
//! Used for efficient convolution via matrix multiplication.

pub async fn unfold(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    kernel_h: usize,
    kernel_w: usize,
    stride: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = (height - kernel_h) / stride + 1;
    let out_w = (width - kernel_w) / stride + 1;
    let num_patches = out_h * out_w;
    let patch_size = channels * kernel_h * kernel_w;

    let mut output = vec![0.0f32; batch_size * patch_size * num_patches];

    for b in 0..batch_size {
        let mut patch_idx = 0;

        for oh in 0..out_h {
            for ow in 0..out_w {
                let mut col_idx = 0;

                for c in 0..channels {
                    for kh in 0..kernel_h {
                        for kw in 0..kernel_w {
                            let ih = oh * stride + kh;
                            let iw = ow * stride + kw;

                            let in_idx = b * channels * height * width
                                + c * height * width
                                + ih * width
                                + iw;
                            let out_idx =
                                b * patch_size * num_patches + col_idx * num_patches + patch_idx;

                            output[out_idx] = input[in_idx];
                            col_idx += 1;
                        }
                    }
                }

                patch_idx += 1;
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
    async fn test_unfold_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Simple 4x4 image, 1 channel, 2x2 kernel, stride 1
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let output = unfold(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 2, 2, 1)
            .await
            .unwrap();

        // Should have (4-2)/1+1 = 3 patches in each dimension = 9 patches total
        // Each patch is 2x2 = 4 values
        assert_eq!(output.len(), 1 * 4 * 9);

        // First patch should be top-left 2x2: [1, 2, 5, 6]
        assert_eq!(output[0], 1.0);
        assert_eq!(output[9], 2.0); // Next column
        assert_eq!(output[18], 5.0); // Next row
        assert_eq!(output[27], 6.0);
    }

    #[tokio::test]
    async fn test_unfold_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Edge case: 3x3 image, 3x3 kernel (single patch)
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let output = unfold(&dev.device, &dev.queue, &input, 1, 1, 3, 3, 3, 3, 1)
            .await
            .unwrap();

        // Single patch of size 9
        assert_eq!(output.len(), 1 * 9 * 1);

        // Should contain all input values
        assert_eq!(output[0], 1.0);
        assert_eq!(output[8], 9.0);
    }

    #[tokio::test]
    async fn test_unfold_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Large kernel with stride 2
        let input = vec![1.0; 1 * 2 * 10 * 10]; // 2 channels, 10x10
        let output = unfold(&dev.device, &dev.queue, &input, 1, 2, 10, 10, 5, 5, 2)
            .await
            .unwrap();

        // (10-5)/2+1 = 3 patches per dimension = 9 patches
        // Each patch: 2 channels * 5*5 = 50 values
        let expected_len = 1 * 50 * 9;
        assert_eq!(output.len(), expected_len);
    }

    #[tokio::test]
    async fn test_unfold_large_tensor() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Realistic image size: 224x224 with 3 channels
        let input = vec![1.0; 1 * 3 * 224 * 224];
        let output = unfold(&dev.device, &dev.queue, &input, 1, 3, 224, 224, 7, 7, 2)
            .await
            .unwrap();

        // (224-7)/2+1 = 109 patches per dimension
        let out_patches = 109 * 109;
        let patch_size = 3 * 7 * 7; // 3 channels, 7x7 kernel

        assert_eq!(output.len(), 1 * patch_size * out_patches);

        // Verify all values are 1.0
        for &val in &output {
            assert_eq!(val, 1.0);
        }
    }

    #[tokio::test]
    async fn test_unfold_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test with distinct values to verify correct extraction
        let input = vec![1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3];
        let output = unfold(&dev.device, &dev.queue, &input, 1, 1, 3, 3, 2, 2, 1)
            .await
            .unwrap();

        // Should have 4 patches (2x2 each)
        let num_patches = 4;
        let patch_size = 4;
        assert_eq!(output.len(), patch_size * num_patches);

        // First patch (top-left 2x2): [1.1, 1.2, 2.1, 2.2]
        assert!((output[0] - 1.1).abs() < 1e-5);
        assert!((output[4] - 1.2).abs() < 1e-5);
        assert!((output[8] - 2.1).abs() < 1e-5);
        assert!((output[12] - 2.2).abs() < 1e-5);
    }
}
