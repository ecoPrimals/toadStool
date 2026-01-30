//! CutMix - CutMix augmentation (Yun et al.)
//!
//! Cuts and pastes patches between training images.
//! Improves generalization and localization.

pub async fn cutmix(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image1: &[f32],
    image2: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    lambda: f32, // Mix ratio
    seed: u64,
) -> Result<(Vec<f32>, f32), Box<dyn std::error::Error>> {
    if image1.len() != image2.len() {
        return Err("Images must have same size".into());
    }
    
    let mut output = image1.to_vec();
    
    // Compute cut size
    let cut_ratio = (1.0 - lambda).sqrt();
    let cut_w = (width as f32 * cut_ratio) as usize;
    let cut_h = (height as f32 * cut_ratio) as usize;
    
    // Random center
    let cx = ((seed * 1103515245) % width as u64) as usize;
    let cy = ((seed * 22695477) % height as u64) as usize;
    
    // Bounding box
    let x1 = (cx as isize - cut_w as isize / 2).max(0) as usize;
    let y1 = (cy as isize - cut_h as isize / 2).max(0) as usize;
    let x2 = (cx + cut_w / 2).min(width);
    let y2 = (cy + cut_h / 2).min(height);
    
    // Copy patch from image2 to output
    for c in 0..channels {
        for i in y1..y2 {
            for j in x1..x2 {
                let idx = c * height * width + i * width + j;
                output[idx] = image2[idx];
            }
        }
    }
    
    // Adjust lambda based on actual cut area
    let actual_lambda = 1.0 - ((x2 - x1) * (y2 - y1)) as f32 / (width * height) as f32;
    
    Ok((output, actual_lambda))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_cutmix() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image1 = vec![1.0; 3 * 32 * 32];
        let image2 = vec![0.5; 3 * 32 * 32];
        let (mixed, lambda) = cutmix(&dev.device, &dev.queue, &image1, &image2, 3, 32, 32, 0.5, 99999).await.unwrap();
        assert_eq!(mixed.len(), image1.len());
        assert!(lambda >= 0.0 && lambda <= 1.0);
    }
}
