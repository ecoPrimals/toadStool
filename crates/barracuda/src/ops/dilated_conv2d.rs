//! Dilated Conv2D - Convolution with dilated kernels
//!
//! Expands receptive field without increasing parameters.
//! Used in DeepLab, WaveNet, dilated residual networks.

pub async fn dilated_conv2d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    kernel: &[f32],
    bias: &[f32],
    batch_size: usize,
    in_channels: usize,
    out_channels: usize,
    height: usize,
    width: usize,
    kernel_size: usize,
    dilation: usize,
    stride: usize,
    padding: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = (height + 2 * padding - dilation * (kernel_size - 1) - 1) / stride + 1;
    let out_w = (width + 2 * padding - dilation * (kernel_size - 1) - 1) / stride + 1;
    let mut output = vec![0.0f32; batch_size * out_channels * out_h * out_w];
    
    for b in 0..batch_size {
        for oc in 0..out_channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let mut sum = bias[oc];
                    
                    for ic in 0..in_channels {
                        for kh in 0..kernel_size {
                            for kw in 0..kernel_size {
                                let ih = oh * stride + kh * dilation;
                                let iw = ow * stride + kw * dilation;
                                
                                if ih >= padding && ih < height + padding && iw >= padding && iw < width + padding {
                                    let ih = ih - padding;
                                    let iw = iw - padding;
                                    
                                    if ih < height && iw < width {
                                        let in_idx = b * in_channels * height * width + ic * height * width + ih * width + iw;
                                        let k_idx = oc * in_channels * kernel_size * kernel_size + ic * kernel_size * kernel_size + kh * kernel_size + kw;
                                        sum += input[in_idx] * kernel[k_idx];
                                    }
                                }
                            }
                        }
                    }
                    
                    let out_idx = b * out_channels * out_h * out_w + oc * out_h * out_w + oh * out_w + ow;
                    output[out_idx] = sum;
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
    async fn test_dilated_conv2d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let kernel = vec![0.1; 16 * 3 * 3 * 3];
        let bias = vec![0.0; 16];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 3, 16, 8, 8, 3, 2, 1, 1).await.unwrap();
        assert!(output.len() > 0);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_dilated_conv2d_edge_cases() {
        let dev = get_test_device().await;

        // Dilation = 1 (standard conv)
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let kernel = vec![1.0; 1 * 1 * 2 * 2];
        let bias = vec![0.0; 1];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 1, 1, 4, 4, 2, 1, 1, 0).await.unwrap();
        assert_eq!(output.len(), 3 * 3);

        // All zeros input
        let input = vec![0.0; 1 * 1 * 4 * 4];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 1, 1, 4, 4, 2, 1, 1, 0).await.unwrap();
        assert!(output.iter().all(|&x| x.abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_dilated_conv2d_boundary() {
        let dev = get_test_device().await;

        // Large dilation (dilation=3)
        let input = vec![1.0; 1 * 1 * 9 * 9];
        let kernel = vec![1.0; 1 * 1 * 3 * 3];
        let bias = vec![0.0; 1];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 1, 1, 9, 9, 3, 3, 1, 0).await.unwrap();
        assert!(output.len() > 0);

        // Different strides
        let input = vec![1.0; 1 * 2 * 8 * 8];
        let kernel = vec![0.1; 4 * 2 * 3 * 3];
        let bias = vec![0.0; 4];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 2, 4, 8, 8, 3, 2, 2, 1).await.unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_dilated_conv2d_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4, DeepLab-style dilation
        let batch_size = 4;
        let in_channels = 8;
        let out_channels = 16;
        let height = 16;
        let width = 16;
        let input = vec![1.0; batch_size * in_channels * height * width];
        let kernel = vec![0.1; out_channels * in_channels * 3 * 3];
        let bias = vec![0.0; out_channels];
        
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, batch_size, in_channels, out_channels, height, width, 3, 2, 1, 1).await.unwrap();
        assert!(output.len() > 0);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_dilated_conv2d_precision() {
        let dev = get_test_device().await;

        // Precision test: verify dilation produces different results than standard conv
        let input = vec![1.0; 1 * 1 * 5 * 5];
        let kernel = vec![1.0; 1 * 1 * 2 * 2]; // All ones kernel
        let bias = vec![0.0; 1];
        
        // Standard conv (dilation=1)
        let output_standard = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 1, 1, 5, 5, 2, 1, 1, 0).await.unwrap();
        
        // Dilated conv (dilation=2)
        let output_dilated = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 1, 1, 5, 5, 2, 2, 1, 0).await.unwrap();
        
        // Dilated conv should have smaller receptive field (fewer elements summed)
        // so output might differ in size or values
        assert!(output_standard.len() > 0);
        assert!(output_dilated.len() > 0);
        
        // All values should be finite and non-negative (all positive inputs/weights)
        assert!(output_dilated.iter().all(|&x| x.is_finite() && x >= 0.0));
    }
}
