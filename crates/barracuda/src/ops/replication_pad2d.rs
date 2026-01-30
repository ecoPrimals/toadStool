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
                    let ih = (oh as isize - pad_top as isize).max(0).min(height as isize - 1) as usize;
                    let iw = (ow as isize - pad_left as isize).max(0).min(width as isize - 1) as usize;
                    
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
    async fn test_replication_pad2d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = replication_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 1, 1, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);
    }
}
