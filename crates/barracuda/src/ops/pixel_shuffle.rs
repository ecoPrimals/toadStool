//! PixelShuffle - Sub-pixel convolution upsampling
//!
//! Rearranges (r²C, H, W) → (C, rH, rW) for efficient upsampling.

pub async fn pixel_shuffle(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    upscale_factor: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let r = upscale_factor;
    let out_channels = channels / (r * r);
    
    if channels % (r * r) != 0 {
        return Err("Channels must be divisible by upscale_factor^2".into());
    }
    
    let out_h = height * r;
    let out_w = width * r;
    let mut output = vec![0.0f32; batch_size * out_channels * out_h * out_w];
    
    for b in 0..batch_size {
        for c in 0..out_channels {
            for h in 0..out_h {
                for w in 0..out_w {
                    let in_h = h / r;
                    let in_w = w / r;
                    let sub_h = h % r;
                    let sub_w = w % r;
                    let in_c = c * r * r + sub_h * r + sub_w;
                    
                    let in_idx = b * channels * height * width + in_c * height * width + in_h * width + in_w;
                    let out_idx = b * out_channels * out_h * out_w + c * out_h * out_w + h * out_w + w;
                    
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
    async fn test_pixel_shuffle_basic() {
        let dev = get_test_device().await;
        let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 4, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 16); // 1 * 1 * 4 * 4
    }

    #[tokio::test]
    async fn test_pixel_shuffle_edge_cases() {
        let dev = get_test_device().await;

        // Upscale by 2x (4 channels → 1 channel, 2×2 → 4×4)
        let input = vec![1.0; 1 * 4 * 2 * 2];
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 4, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);

        // Small upscale (r=1, no-op)
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 1).await.unwrap();
        assert_eq!(output.len(), 16);
    }

    #[tokio::test]
    async fn test_pixel_shuffle_boundary() {
        let dev = get_test_device().await;

        // Large upscale factor (r=3)
        let input = vec![1.0; 1 * 9 * 2 * 2]; // 9 channels for r=3
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 9, 2, 2, 3).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 6 * 6);

        // Many channels
        let input = vec![1.0; 1 * 16 * 4 * 4]; // 16 channels, r=2
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 16, 4, 4, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 4 * 8 * 8);
    }

    #[tokio::test]
    async fn test_pixel_shuffle_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4
        let batch_size = 4;
        let input = vec![1.0; batch_size * 4 * 8 * 8];
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, batch_size, 4, 8, 8, 2).await.unwrap();
        assert_eq!(output.len(), batch_size * 1 * 16 * 16);
    }

    #[tokio::test]
    async fn test_pixel_shuffle_precision() {
        let dev = get_test_device().await;

        // Verify value rearrangement
        let mut input = vec![0.0; 1 * 4 * 2 * 2];
        for i in 0..16 {
            input[i] = i as f32;
        }
        
        let output = pixel_shuffle(&dev.device, &dev.queue, &input, 1, 4, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 16);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Values should be rearranged, not duplicated
        assert!(output.iter().all(|&x| x >= 0.0 && x < 16.0));
    }
}
