//! GridMask - Grid-based masking augmentation (Chen et al.)
//!
//! Masks structured grid regions in images.
//! Prevents overfitting to spatial structures.

pub async fn grid_mask(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    ratio: f32,      // Mask ratio (0.0 to 1.0)
    rotate: f32,     // Rotation angle in degrees
    grid_size: usize,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = image.to_vec();
    
    // Random offset
    let offset_x = ((seed * 1103515245) % grid_size as u64) as usize;
    let offset_y = ((seed * 22695477) % grid_size as u64) as usize;
    
    let mask_size = (grid_size as f32 * ratio) as usize;
    let angle_rad = rotate * std::f32::consts::PI / 180.0;
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    
    // Apply grid mask
    for c in 0..channels {
        for i in 0..height {
            for j in 0..width {
                // Rotate coordinates
                let x = j as f32 - cx;
                let y = i as f32 - cy;
                let rot_x = (x * cos_a - y * sin_a + cx) as isize;
                let rot_y = (x * sin_a + y * cos_a + cy) as isize;
                
                if rot_x >= 0 && rot_x < width as isize 
                && rot_y >= 0 && rot_y < height as isize {
                    let grid_x = ((rot_x as usize + offset_x) / grid_size) % 2;
                    let grid_y = ((rot_y as usize + offset_y) / grid_size) % 2;
                    
                    // Mask alternating grid cells
                    if (grid_x + grid_y) % 2 == 0 {
                        let local_x = (rot_x as usize + offset_x) % grid_size;
                        let local_y = (rot_y as usize + offset_y) % grid_size;
                        
                        if local_x < mask_size && local_y < mask_size {
                            output[c * height * width + i * width + j] = 0.0;
                        }
                    }
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
    async fn test_grid_mask() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![1.0; 3 * 224 * 224];
        let masked = grid_mask(&dev.device, &dev.queue, &image, 3, 224, 224, 0.6, 15.0, 96, 11111).await.unwrap();
        assert_eq!(masked.len(), image.len());
    }
}
