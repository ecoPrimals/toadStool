//! Mosaic - Mosaic augmentation (YOLO-style)
//!
//! Combines 4 images into one mosaic.
//! Used in object detection for multi-scale training.

pub async fn mosaic(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    images: &[Vec<f32>], // 4 images [C, H, W]
    channels: usize,
    height: usize,
    width: usize,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if images.len() != 4 {
        return Err("Mosaic requires exactly 4 images".into());
    }
    
    for img in images {
        if img.len() != channels * height * width {
            return Err("All images must have same dimensions".into());
        }
    }
    
    let mut output = vec![0.0f32; channels * height * width];
    
    // Random split point
    let split_x = ((seed * 1103515245) % width as u64) as usize;
    let split_y = ((seed * 22695477) % height as u64) as usize;
    
    // Top-left: image 0
    for c in 0..channels {
        for i in 0..split_y {
            for j in 0..split_x {
                let src_idx = c * height * width + i * width + j;
                let dst_idx = c * height * width + i * width + j;
                output[dst_idx] = images[0][src_idx];
            }
        }
    }
    
    // Top-right: image 1
    for c in 0..channels {
        for i in 0..split_y {
            for j in split_x..width {
                let src_idx = c * height * width + i * width + j;
                let dst_idx = c * height * width + i * width + j;
                output[dst_idx] = images[1][src_idx];
            }
        }
    }
    
    // Bottom-left: image 2
    for c in 0..channels {
        for i in split_y..height {
            for j in 0..split_x {
                let src_idx = c * height * width + i * width + j;
                let dst_idx = c * height * width + i * width + j;
                output[dst_idx] = images[2][src_idx];
            }
        }
    }
    
    // Bottom-right: image 3
    for c in 0..channels {
        for i in split_y..height {
            for j in split_x..width {
                let src_idx = c * height * width + i * width + j;
                let dst_idx = c * height * width + i * width + j;
                output[dst_idx] = images[3][src_idx];
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
    async fn test_mosaic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let images = vec![
            vec![1.0; 3 * 640 * 640],
            vec![0.8; 3 * 640 * 640],
            vec![0.6; 3 * 640 * 640],
            vec![0.4; 3 * 640 * 640],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 3, 640, 640, 77777).await.unwrap();
        assert_eq!(mosaic_img.len(), 3 * 640 * 640);
    }
}
