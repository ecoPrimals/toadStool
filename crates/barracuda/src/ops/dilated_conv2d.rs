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
    
    #[tokio::test]
    async fn test_dilated_conv2d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let kernel = vec![0.1; 16 * 3 * 3 * 3];
        let bias = vec![0.0; 16];
        let output = dilated_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 3, 16, 8, 8, 3, 2, 1, 1).await.unwrap();
        assert!(output.len() > 0);
    }
}
