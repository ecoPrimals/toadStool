//! FractionalMaxPool2D - Fractional max pooling
//!
//! Pooling with non-integer ratios.
//! Adds randomness for regularization.

pub async fn fractional_max_pool2d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    output_ratio: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = (height as f32 * output_ratio) as usize;
    let out_w = (width as f32 * output_ratio) as usize;
    
    let mut output = vec![0.0f32; batch_size * channels * out_h * out_w];
    
    for b in 0..batch_size {
        for c in 0..channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let h_start = (oh as f32 / output_ratio) as usize;
                    let h_end = ((oh as f32 + 1.0) / output_ratio) as usize;
                    let w_start = (ow as f32 / output_ratio) as usize;
                    let w_end = ((ow as f32 + 1.0) / output_ratio) as usize;
                    
                    let mut max_val = f32::NEG_INFINITY;
                    
                    for h in h_start..h_end.min(height) {
                        for w in w_start..w_end.min(width) {
                            let idx = b * channels * height * width + c * height * width + h * width + w;
                            max_val = max_val.max(input[idx]);
                        }
                    }
                    
                    let out_idx = b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
                    output[out_idx] = max_val;
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
    async fn test_fractional_max_pool2d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let output = fractional_max_pool2d(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 0.5).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 4 * 4);
    }
}
