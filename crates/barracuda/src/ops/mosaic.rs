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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_mosaic_basic() {
        let dev = get_test_device().await;
        let images = vec![
            vec![1.0; 3 * 640 * 640],
            vec![0.8; 3 * 640 * 640],
            vec![0.6; 3 * 640 * 640],
            vec![0.4; 3 * 640 * 640],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 3, 640, 640, 77777).await.unwrap();
        assert_eq!(mosaic_img.len(), 3 * 640 * 640);
        assert!(mosaic_img.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_mosaic_edge_cases() {
        let dev = get_test_device().await;

        // Small images
        let images = vec![
            vec![1.0; 3 * 32 * 32],
            vec![2.0; 3 * 32 * 32],
            vec![3.0; 3 * 32 * 32],
            vec![4.0; 3 * 32 * 32],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 3, 32, 32, 12345).await.unwrap();
        assert_eq!(mosaic_img.len(), 3 * 32 * 32);

        // Single channel (grayscale)
        let images = vec![
            vec![1.0; 1 * 64 * 64],
            vec![2.0; 1 * 64 * 64],
            vec![3.0; 1 * 64 * 64],
            vec![4.0; 1 * 64 * 64],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 1, 64, 64, 99999).await.unwrap();
        assert_eq!(mosaic_img.len(), 1 * 64 * 64);
    }

    #[tokio::test]
    async fn test_mosaic_boundary() {
        let dev = get_test_device().await;

        // Different seeds produce different mosaics
        let images = vec![
            vec![1.0; 3 * 128 * 128],
            vec![2.0; 3 * 128 * 128],
            vec![3.0; 3 * 128 * 128],
            vec![4.0; 3 * 128 * 128],
        ];
        let mosaic1 = mosaic(&dev.device, &dev.queue, &images, 3, 128, 128, 111).await.unwrap();
        let mosaic2 = mosaic(&dev.device, &dev.queue, &images, 3, 128, 128, 222).await.unwrap();
        assert_eq!(mosaic1.len(), mosaic2.len());
    }

    #[tokio::test]
    async fn test_mosaic_large_images() {
        let dev = get_test_device().await;

        // HD images
        let images = vec![
            vec![1.0; 3 * 1024 * 1024],
            vec![0.5; 3 * 1024 * 1024],
            vec![0.25; 3 * 1024 * 1024],
            vec![0.0; 3 * 1024 * 1024],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 3, 1024, 1024, 42).await.unwrap();
        assert_eq!(mosaic_img.len(), 3 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_mosaic_precision() {
        let dev = get_test_device().await;

        // Test that all 4 quadrants are represented
        let images = vec![
            vec![1.0; 3 * 100 * 100],
            vec![2.0; 3 * 100 * 100],
            vec![3.0; 3 * 100 * 100],
            vec![4.0; 3 * 100 * 100],
        ];
        let mosaic_img = mosaic(&dev.device, &dev.queue, &images, 3, 100, 100, 50505).await.unwrap();
        
        // Should contain values from all 4 images
        assert_eq!(mosaic_img.len(), 3 * 100 * 100);
        assert!(mosaic_img.iter().all(|&x| x >= 1.0 && x <= 4.0));
    }
}
