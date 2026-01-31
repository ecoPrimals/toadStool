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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_random_crop_basic() {
        let dev = get_test_device().await;
        let image = vec![1.0; 3 * 32 * 32];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 32, 32, 24, 24, 4, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 24 * 24);
    }

    #[tokio::test]
    async fn test_random_crop_edge_cases() {
        let dev = get_test_device().await;

        // No padding
        let image = vec![1.0; 3 * 32 * 32];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 32, 32, 16, 16, 0, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 16 * 16);

        // Full image (crop = input size)
        let image = vec![1.0; 3 * 8 * 8];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 8, 8, 8, 8, 0, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 8 * 8);
    }

    #[tokio::test]
    async fn test_random_crop_boundary() {
        let dev = get_test_device().await;

        // Large padding
        let image = vec![1.0; 3 * 16 * 16];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 16, 16, 20, 20, 4, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 20 * 20);

        // Single channel
        let image = vec![1.0; 1 * 32 * 32];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 1, 32, 32, 24, 24, 4, 12345).await.unwrap();
        assert_eq!(cropped.len(), 24 * 24);
    }

    #[tokio::test]
    async fn test_random_crop_large_batch() {
        let dev = get_test_device().await;

        // High resolution
        let image = vec![1.0; 3 * 256 * 256];
        let cropped = random_crop(&dev.device, &dev.queue, &image, 3, 256, 256, 224, 224, 4, 12345).await.unwrap();
        assert_eq!(cropped.len(), 3 * 224 * 224);
    }

    #[tokio::test]
    async fn test_random_crop_precision() {
        let dev = get_test_device().await;

        // Verify deterministic with same seed
        let image = vec![1.0; 3 * 32 * 32];
        let cropped1 = random_crop(&dev.device, &dev.queue, &image, 3, 32, 32, 16, 16, 4, 12345).await.unwrap();
        let cropped2 = random_crop(&dev.device, &dev.queue, &image, 3, 32, 32, 16, 16, 4, 12345).await.unwrap();
        assert_eq!(cropped1, cropped2);
        assert_eq!(cropped1.len(), 3 * 16 * 16);
    }
}
