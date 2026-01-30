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
    
    #[tokio::test]
    async fn test_pixel_shuffle() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let output = pixel_shuffle(&device, &queue, &input, 1, 4, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 16); // 1 * 1 * 4 * 4
    }
}
