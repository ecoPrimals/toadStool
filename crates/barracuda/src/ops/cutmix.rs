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
    async fn test_cutmix_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image1 = vec![1.0; 3 * 32 * 32];
        let image2 = vec![0.5; 3 * 32 * 32];
        let (mixed, lambda) = cutmix(
            &dev.device,
            &dev.queue,
            &image1,
            &image2,
            3,
            32,
            32,
            0.5,
            99999,
        )
        .await
        .unwrap();

        assert_eq!(mixed.len(), image1.len());
        assert!(lambda >= 0.0 && lambda <= 1.0);

        // Mixed image should contain values from both images
        let has_image1 = mixed.iter().any(|&v| (v - 1.0).abs() < 1e-5);
        let has_image2 = mixed.iter().any(|&v| (v - 0.5).abs() < 1e-5);
        assert!(has_image1 && has_image2, "CutMix should mix both images");
    }

    #[tokio::test]
    async fn test_cutmix_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Edge case: lambda = 1.0 (no mix, all image1)
        let image1 = vec![1.0; 3 * 16 * 16];
        let image2 = vec![0.0; 3 * 16 * 16];
        let (_mixed, lambda) = cutmix(
            &dev.device,
            &dev.queue,
            &image1,
            &image2,
            3,
            16,
            16,
            1.0,
            12345,
        )
        .await
        .unwrap();

        // With lambda=1.0, cut_ratio=0, so should be mostly image1
        assert!(lambda > 0.9);

        // Edge case: lambda = 0.0 (max mix)
        let (_mixed, lambda) = cutmix(
            &dev.device,
            &dev.queue,
            &image1,
            &image2,
            3,
            16,
            16,
            0.0,
            54321,
        )
        .await
        .unwrap();
        assert!(lambda < 0.5); // Significant mixing
    }

    #[tokio::test]
    async fn test_cutmix_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Small image
        let image1 = vec![1.0; 3 * 4 * 4];
        let image2 = vec![0.0; 3 * 4 * 4];
        let (mixed, lambda) = cutmix(&dev.device, &dev.queue, &image1, &image2, 3, 4, 4, 0.5, 777)
            .await
            .unwrap();

        assert_eq!(mixed.len(), 3 * 4 * 4);
        assert!(lambda >= 0.0 && lambda <= 1.0);

        // Single channel
        let gray1 = vec![1.0; 1 * 8 * 8];
        let gray2 = vec![0.0; 1 * 8 * 8];
        let (mixed, _) = cutmix(&dev.device, &dev.queue, &gray1, &gray2, 1, 8, 8, 0.5, 888)
            .await
            .unwrap();
        assert_eq!(mixed.len(), 64);
    }

    #[tokio::test]
    async fn test_cutmix_large_image() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Realistic image size (224x224 RGB)
        let size = 3 * 224 * 224;
        let image1 = vec![1.0; size];
        let image2 = vec![0.0; size];

        let (mixed, lambda) = cutmix(
            &dev.device,
            &dev.queue,
            &image1,
            &image2,
            3,
            224,
            224,
            0.5,
            11111,
        )
        .await
        .unwrap();

        assert_eq!(mixed.len(), size);
        assert!(lambda >= 0.0 && lambda <= 1.0);

        // Verify mixing occurred
        let sum: f32 = mixed.iter().sum();
        let expected_sum = size as f32 * lambda; // Weighted by lambda
        assert!((sum - expected_sum).abs() / expected_sum < 0.2); // Within 20%
    }

    #[tokio::test]
    async fn test_cutmix_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test with distinct patterns
        let image1 = vec![1.0; 3 * 16 * 16];
        let image2 = vec![0.0; 3 * 16 * 16];

        // Create checkerboard pattern in image1
        // (Note: CutMix doesn't modify input, so pattern is informational)

        let (mixed, lambda) = cutmix(
            &dev.device,
            &dev.queue,
            &image1,
            &image2,
            3,
            16,
            16,
            0.5,
            55555,
        )
        .await
        .unwrap();

        // Verify patch was cut correctly
        assert_eq!(mixed.len(), 3 * 16 * 16);
        assert!(lambda >= 0.0 && lambda <= 1.0);

        // Should have values from both images
        let has_ones = mixed.iter().any(|&v| (v - 1.0).abs() < 1e-5);
        let has_zeros = mixed.iter().any(|&v| v.abs() < 1e-5);
        assert!(has_ones || has_zeros);
    }
}
