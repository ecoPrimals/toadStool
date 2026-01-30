//! ReflectionPad2D - Padding with reflection
//!
//! Pads image by reflecting pixels at borders.
//! Better for image tasks than zero-padding.

pub async fn reflection_pad2d(
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
                    // Map output coord to input coord with reflection
                    let mut ih = oh as isize - pad_top as isize;
                    let mut iw = ow as isize - pad_left as isize;
                    
                    // Reflect if outside bounds
                    if ih < 0 {
                        ih = -ih;
                    } else if ih >= height as isize {
                        ih = 2 * (height as isize - 1) - ih;
                    }
                    
                    if iw < 0 {
                        iw = -iw;
                    } else if iw >= width as isize {
                        iw = 2 * (width as isize - 1) - iw;
                    }
                    
                    let ih = ih.max(0).min(height as isize - 1) as usize;
                    let iw = iw.max(0).min(width as isize - 1) as usize;
                    
                    let in_idx = b * channels * height * width + c * height * width + ih * width + iw;
                    let out_idx = b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
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
    
    #[tokio::test]
    async fn test_reflection_pad2d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0, 4.0]; // 1x1x2x2
        let output = reflection_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 1, 1, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);
    }
}
