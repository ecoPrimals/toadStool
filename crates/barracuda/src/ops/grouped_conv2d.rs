//! Grouped Conv2D - Convolution with channel groups
//!
//! Divides input/output channels into groups.
//! Reduces parameters, used in ResNeXt, ShuffleNet.

pub async fn grouped_conv2d(
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
    groups: usize,
    stride: usize,
    padding: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if in_channels % groups != 0 || out_channels % groups != 0 {
        return Err("Channels must be divisible by groups".into());
    }
    
    let in_per_group = in_channels / groups;
    let out_per_group = out_channels / groups;
    let out_h = (height + 2 * padding - kernel_size) / stride + 1;
    let out_w = (width + 2 * padding - kernel_size) / stride + 1;
    let mut output = vec![0.0f32; batch_size * out_channels * out_h * out_w];
    
    for b in 0..batch_size {
        for g in 0..groups {
            for oc_local in 0..out_per_group {
                let oc = g * out_per_group + oc_local;
                
                for oh in 0..out_h {
                    for ow in 0..out_w {
                        let mut sum = bias[oc];
                        
                        for ic_local in 0..in_per_group {
                            let ic = g * in_per_group + ic_local;
                            
                            for kh in 0..kernel_size {
                                for kw in 0..kernel_size {
                                    let ih = oh * stride + kh;
                                    let iw = ow * stride + kw;
                                    
                                    if ih >= padding && ih < height + padding && iw >= padding && iw < width + padding {
                                        let ih = ih - padding;
                                        let iw = iw - padding;
                                        
                                        if ih < height && iw < width {
                                            let in_idx = b * in_channels * height * width + ic * height * width + ih * width + iw;
                                            let k_idx = oc_local * in_per_group * kernel_size * kernel_size + ic_local * kernel_size * kernel_size + kh * kernel_size + kw;
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
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_grouped_conv2d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 4 * 8 * 8];
        let kernel = vec![0.1; 2 * 2 * 3 * 3]; // 8 out / 2 groups = 4 kernels per group
        let bias = vec![0.0; 8];
        let output = grouped_conv2d(&dev.device, &dev.queue, &input, &kernel, &bias, 1, 4, 8, 8, 8, 3, 2, 1, 1).await.unwrap();
        assert!(output.len() > 0);
    }
}
