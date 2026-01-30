//! RandomCrop - Random cropping with padding
//!
//! Randomly crops image with optional padding.
//! Standard augmentation for image classification.

pub async fn random_crop(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32], // [C, H, W]
    channels: usize,
    height: usize,
    width: usize,
    crop_h: usize,
    crop_w: usize,
    padding: usize,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if crop_h > height + 2 * padding || crop_w > width + 2 * padding {
        return Err("Crop size exceeds padded image size".into());
    }
    
    // Simplified random using seed
    let rng_h = ((seed * 1664525 + 1013904223) % ((height + 2 * padding - crop_h + 1) as u64)) as usize;
    let rng_w = ((seed * 22695477 + 1) % ((width + 2 * padding - crop_w + 1) as u64)) as usize;
    
    let mut output = vec![0.0f32; channels * crop_h * crop_w];
    
    for c in 0..channels {
        for i in 0..crop_h {
            for j in 0..crop_w {
                let src_i = (rng_h + i) as isize - padding as isize;
                let src_j = (rng_w + j) as isize - padding as isize;
                
                let val = if src_i >= 0 && src_i < height as isize 
                          && src_j >= 0 && src_j < width as isize {
                    image[c * height * width + src_i as usize * width + src_j as usize]
                } else {
                    0.0 // Zero padding
                };
                
                output[c * crop_h * crop_w + i * crop_w + j] = val;
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
    async fn test_random_crop() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![1.0; 3 * 32 * 32];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 32, 32, 24, 24, 4, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 24 * 24);
    }
}
