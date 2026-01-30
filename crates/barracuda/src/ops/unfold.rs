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
                            
                            let in_idx = b * channels * height * width + c * height * width + ih * width + iw;
                            let out_idx = b * patch_size * num_patches + col_idx * num_patches + patch_idx;
                            
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
    async fn test_unfold() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let output = unfold(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 3, 3, 1).await.unwrap();
        assert!(output.len() > 0);
    }
}
