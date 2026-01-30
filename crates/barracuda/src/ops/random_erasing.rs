//! RandomErasing - Random erasing augmentation
//!
//! Randomly masks rectangular regions in images.
//! Improves robustness and prevents overfitting.

pub async fn random_erasing(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    probability: f32,
    scale_min: f32,
    scale_max: f32,
    aspect_min: f32,
    aspect_max: f32,
    value: f32,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = image.to_vec();
    
    // Check if we should erase (based on probability)
    let rand_val = ((seed * 1103515245 + 12345) % 1000) as f32 / 1000.0;
    if rand_val > probability {
        return Ok(output);
    }
    
    // Random scale
    let scale_rand = ((seed * 22695477) % 1000) as f32 / 1000.0;
    let scale = scale_min + (scale_max - scale_min) * scale_rand;
    let erase_area = (height * width) as f32 * scale;
    
    // Random aspect ratio
    let aspect_rand = ((seed * 1664525) % 1000) as f32 / 1000.0;
    let aspect = aspect_min + (aspect_max - aspect_min) * aspect_rand;
    
    let erase_h = (erase_area * aspect).sqrt() as usize;
    let erase_w = (erase_area / aspect).sqrt() as usize;
    
    if erase_h >= height || erase_w >= width {
        return Ok(output);
    }
    
    // Random position
    let pos_h = ((seed * 48271) % (height - erase_h) as u64) as usize;
    let pos_w = ((seed * 69621) % (width - erase_w) as u64) as usize;
    
    // Erase region
    for c in 0..channels {
        for i in pos_h..(pos_h + erase_h).min(height) {
            for j in pos_w..(pos_w + erase_w).min(width) {
                output[c * height * width + i * width + j] = value;
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
    async fn test_random_erasing() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![1.0; 3 * 224 * 224];
        let erased = random_erasing(&dev.device, &dev.queue, &image, 3, 224, 224, 0.5, 0.02, 0.4, 0.3, 3.0, 0.0, 54321).await.unwrap();
        assert_eq!(erased.len(), image.len());
    }
}
