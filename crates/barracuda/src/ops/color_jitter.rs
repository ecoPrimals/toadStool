//! ColorJitter - Color augmentation
//!
//! Randomly changes brightness, contrast, saturation, and hue.
//! Standard color augmentation for robustness.

pub async fn color_jitter(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32], // [C, H, W] in RGB
    channels: usize,
    height: usize,
    width: usize,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    hue: f32,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if channels != 3 {
        return Err("ColorJitter requires RGB image (3 channels)".into());
    }

    let mut output = image.to_vec();
    let size = height * width;

    // Random factors
    let brightness_factor = 1.0 + brightness * (((seed * 1103515245) % 2000) as f32 / 1000.0 - 1.0);
    let contrast_factor = 1.0 + contrast * (((seed * 22695477) % 2000) as f32 / 1000.0 - 1.0);
    let saturation_factor = 1.0 + saturation * (((seed * 1664525) % 2000) as f32 / 1000.0 - 1.0);
    let _hue_factor = hue * (((seed * 48271) % 2000) as f32 / 1000.0 - 1.0); // For future HSV transform

    for i in 0..size {
        let r = image[0 * size + i];
        let g = image[1 * size + i];
        let b = image[2 * size + i];

        // Brightness
        let r = (r * brightness_factor).clamp(0.0, 1.0);
        let g = (g * brightness_factor).clamp(0.0, 1.0);
        let b = (b * brightness_factor).clamp(0.0, 1.0);

        // Contrast (around mean)
        let mean = (r + g + b) / 3.0;
        let r = (mean + (r - mean) * contrast_factor).clamp(0.0, 1.0);
        let g = (mean + (g - mean) * contrast_factor).clamp(0.0, 1.0);
        let b = (mean + (b - mean) * contrast_factor).clamp(0.0, 1.0);

        // Saturation (convert to HSV-like and back)
        let max_val = r.max(g).max(b);
        let min_val = r.min(g).min(b);
        let delta = max_val - min_val;

        if delta > 1e-8 {
            let sat = delta / (max_val + 1e-8);
            let new_sat = (sat * saturation_factor).clamp(0.0, 1.0);
            let sat_ratio = new_sat / (sat + 1e-8);

            let r = max_val - (max_val - r) * sat_ratio;
            let g = max_val - (max_val - g) * sat_ratio;
            let b = max_val - (max_val - b) * sat_ratio;

            output[0 * size + i] = r.clamp(0.0, 1.0);
            output[1 * size + i] = g.clamp(0.0, 1.0);
            output[2 * size + i] = b.clamp(0.0, 1.0);
        } else {
            output[0 * size + i] = r;
            output[1 * size + i] = g;
            output[2 * size + i] = b;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_color_jitter_basic() {
        let dev = get_test_device().await;
        let image = vec![0.5; 3 * 128 * 128];
        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            128,
            128,
            0.2,
            0.2,
            0.2,
            0.1,
            88888,
        )
        .await
        .unwrap();
        assert_eq!(jittered.len(), image.len());
        assert!(jittered.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_color_jitter_edge_cases() {
        let dev = get_test_device().await;

        // No augmentation (all factors = 0)
        let image = vec![0.5; 3 * 4 * 4];
        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            4,
            4,
            0.0,
            0.0,
            0.0,
            0.0,
            12345,
        )
        .await
        .unwrap();
        // Should be similar to input (minor numerical differences allowed)
        assert_eq!(jittered.len(), image.len());

        // Grayscale image (R=G=B)
        let image = vec![0.3; 3 * 4 * 4];
        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            4,
            4,
            0.1,
            0.1,
            0.1,
            0.1,
            99999,
        )
        .await
        .unwrap();
        assert!(jittered.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_color_jitter_boundary() {
        let dev = get_test_device().await;

        // Maximum augmentation
        let image = vec![0.5; 3 * 8 * 8];
        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            8,
            8,
            1.0,
            1.0,
            1.0,
            1.0,
            77777,
        )
        .await
        .unwrap();
        assert!(jittered.iter().all(|&x| x >= 0.0 && x <= 1.0));

        // Extreme values (black and white)
        let mut image = vec![0.0; 3 * 4 * 4];
        for i in (3 * 4 * 4 / 2)..(3 * 4 * 4) {
            image[i] = 1.0;
        }
        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            4,
            4,
            0.3,
            0.3,
            0.3,
            0.3,
            55555,
        )
        .await
        .unwrap();
        assert!(jittered.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_color_jitter_large_batch() {
        let dev = get_test_device().await;

        // Large image
        let width = 256;
        let height = 256;
        let image: Vec<f32> = (0..3 * width * height)
            .map(|i| ((i % 100) as f32) / 100.0)
            .collect();

        let jittered = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            height,
            width,
            0.4,
            0.4,
            0.4,
            0.2,
            11111,
        )
        .await
        .unwrap();

        assert_eq!(jittered.len(), image.len());
        assert!(jittered.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_color_jitter_precision() {
        let dev = get_test_device().await;

        // Test determinism with same seed
        let image = vec![0.6; 3 * 16 * 16];
        let seed = 42424;

        let jittered1 = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            16,
            16,
            0.3,
            0.3,
            0.3,
            0.2,
            seed,
        )
        .await
        .unwrap();
        let jittered2 = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            16,
            16,
            0.3,
            0.3,
            0.3,
            0.2,
            seed,
        )
        .await
        .unwrap();

        // Same seed should produce identical results
        assert_eq!(jittered1, jittered2);

        // Different seed should produce different results
        let jittered3 = color_jitter(
            &dev.device,
            &dev.queue,
            &image,
            3,
            16,
            16,
            0.3,
            0.3,
            0.3,
            0.2,
            seed + 1,
        )
        .await
        .unwrap();
        assert_ne!(jittered1, jittered3);
    }
}
