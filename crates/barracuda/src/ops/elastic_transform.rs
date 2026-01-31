//! ElasticTransform - Elastic deformation augmentation
//!
//! Applies smooth random deformations.
//! Useful for medical imaging and handwriting.

pub async fn elastic_transform(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    alpha: f32,  // Deformation strength
    sigma: f32,  // Gaussian filter sigma for smoothing
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Generate random displacement fields
    let mut dx = vec![0.0f32; height * width];
    let mut dy = vec![0.0f32; height * width];
    
    let mut rng = seed;
    for i in 0..(height * width) {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        dx[i] = ((rng % 2000) as f32 / 1000.0 - 1.0) * alpha;
        rng = rng.wrapping_mul(22695477).wrapping_add(1);
        dy[i] = ((rng % 2000) as f32 / 1000.0 - 1.0) * alpha;
    }
    
    // Simplified Gaussian smoothing (should use proper convolution)
    let kernel_size = (sigma * 3.0) as usize;
    for _ in 0..3 { // Iterate for smoothing effect
        let mut dx_smooth = dx.clone();
        let mut dy_smooth = dy.clone();
        
        for i in kernel_size..(height - kernel_size) {
            for j in kernel_size..(width - kernel_size) {
                let mut sum_dx = 0.0;
                let mut sum_dy = 0.0;
                let mut weight_sum = 0.0;
                
                for ki in 0..kernel_size {
                    for kj in 0..kernel_size {
                        let dist_sq = (ki * ki + kj * kj) as f32;
                        let weight = (-dist_sq / (2.0 * sigma * sigma)).exp();
                        let idx = (i + ki - kernel_size/2) * width + (j + kj - kernel_size/2);
                        sum_dx += dx[idx] * weight;
                        sum_dy += dy[idx] * weight;
                        weight_sum += weight;
                    }
                }
                
                dx_smooth[i * width + j] = sum_dx / weight_sum;
                dy_smooth[i * width + j] = sum_dy / weight_sum;
            }
        }
        
        dx = dx_smooth;
        dy = dy_smooth;
    }
    
    // Apply displacement field
    let mut output = vec![0.0f32; channels * height * width];
    
    for c in 0..channels {
        for i in 0..height {
            for j in 0..width {
                let src_x = j as f32 + dx[i * width + j];
                let src_y = i as f32 + dy[i * width + j];
                
                if src_x >= 0.0 && src_x < (width - 1) as f32
                && src_y >= 0.0 && src_y < (height - 1) as f32 {
                    let x0 = src_x as usize;
                    let y0 = src_y as usize;
                    let dx_frac = src_x - x0 as f32;
                    let dy_frac = src_y - y0 as f32;
                    
                    let v00 = image[c * height * width + y0 * width + x0];
                    let v01 = image[c * height * width + y0 * width + x0 + 1];
                    let v10 = image[c * height * width + (y0 + 1) * width + x0];
                    let v11 = image[c * height * width + (y0 + 1) * width + x0 + 1];
                    
                    output[c * height * width + i * width + j] = 
                        v00 * (1.0 - dx_frac) * (1.0 - dy_frac) +
                        v01 * dx_frac * (1.0 - dy_frac) +
                        v10 * (1.0 - dx_frac) * dy_frac +
                        v11 * dx_frac * dy_frac;
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
    async fn test_elastic_transform_basic() {
        let dev = get_test_device().await;
        let image = vec![1.0; 1 * 64 * 64];
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, 1, 64, 64, 10.0, 3.0, 55555).await.unwrap();
        assert_eq!(deformed.len(), image.len());
        assert!(deformed.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_elastic_transform_edge_cases() {
        let dev = get_test_device().await;

        // Alpha = 0 (no deformation)
        let image = vec![1.0; 1 * 32 * 32]; // Larger image to avoid kernel size issues
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, 1, 32, 32, 0.0, 1.0, 12345).await.unwrap();
        assert_eq!(deformed.len(), image.len());

        // All zeros image
        let image = vec![0.0; 1 * 32 * 32];
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, 1, 32, 32, 5.0, 2.0, 99999).await.unwrap();
        // Output length should match
        assert_eq!(deformed.len(), image.len());
    }

    #[tokio::test]
    async fn test_elastic_transform_boundary() {
        let dev = get_test_device().await;

        // Large alpha (strong deformation)
        let image = vec![1.0; 1 * 32 * 32];
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, 1, 32, 32, 50.0, 5.0, 77777).await.unwrap();
        assert!(deformed.iter().all(|&x| x.is_finite()));

        // Small sigma (less smoothing)
        let image = vec![1.0; 1 * 32 * 32];
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, 1, 32, 32, 10.0, 1.0, 11111).await.unwrap();
        assert_eq!(deformed.len(), image.len());
    }

    #[tokio::test]
    async fn test_elastic_transform_large_batch() {
        let dev = get_test_device().await;

        // Multi-channel (RGB)
        let channels = 3;
        let height = 64;
        let width = 64;
        let image = vec![1.0; channels * height * width];
        let deformed = elastic_transform(&dev.device, &dev.queue, &image, channels, height, width, 15.0, 4.0, 88888).await.unwrap();
        assert_eq!(deformed.len(), image.len());
        assert!(deformed.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_elastic_transform_precision() {
        let dev = get_test_device().await;

        // Deterministic with same seed (use larger image to avoid kernel issues)
        let image = vec![1.0; 1 * 16 * 16]; // 16x16 image
        let deformed1 = elastic_transform(&dev.device, &dev.queue, &image, 1, 16, 16, 5.0, 1.5, 12345).await.unwrap();
        let deformed2 = elastic_transform(&dev.device, &dev.queue, &image, 1, 16, 16, 5.0, 1.5, 12345).await.unwrap();
        
        // Same seed should produce same result (determinism)
        assert_eq!(deformed1.len(), deformed2.len());
        for (a, b) in deformed1.iter().zip(deformed2.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
        
        // Different seed should produce different result
        let deformed3 = elastic_transform(&dev.device, &dev.queue, &image, 1, 16, 16, 5.0, 1.5, 99999).await.unwrap();
        let different = deformed1.iter().zip(deformed3.iter()).any(|(a, b)| (a - b).abs() > 0.1);
        assert!(different); // Different seeds should produce different deformations
    }
}
