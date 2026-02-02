//! PSNR - Peak Signal-to-Noise Ratio
//!
//! Measures reconstruction quality in dB.
//! Higher is better (typically 30-50 dB for good quality).

pub async fn psnr(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    original: &[f32],
    reconstructed: &[f32],
    max_pixel_value: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if original.len() != reconstructed.len() {
        return Err("Arrays must have same length".into());
    }

    if original.is_empty() {
        return Err("Empty arrays".into());
    }

    // Compute MSE
    let mut mse = 0.0;
    for i in 0..original.len() {
        let diff = original[i] - reconstructed[i];
        mse += diff * diff;
    }
    mse /= original.len() as f32;

    if mse < 1e-10 {
        return Ok(f32::INFINITY); // Perfect reconstruction
    }

    // PSNR = 10 * log10(MAX^2 / MSE)
    let psnr_val = 10.0 * (max_pixel_value * max_pixel_value / mse).log10();

    Ok(psnr_val)
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
    async fn test_psnr_basic() {
        let dev = get_test_device().await;
        let original = vec![0.5; 1000];
        let reconstructed = vec![0.5; 1000];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0)
            .await
            .unwrap();
        assert!(psnr_val > 100.0); // Should be very high for identical signals
    }

    #[tokio::test]
    async fn test_psnr_edge_cases() {
        let dev = get_test_device().await;

        // Perfect reconstruction
        let original = vec![0.1, 0.5, 0.9];
        let reconstructed = vec![0.1, 0.5, 0.9];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0)
            .await
            .unwrap();
        assert!(psnr_val.is_infinite()); // MSE ~= 0

        // Significant difference (low PSNR)
        let original = vec![1.0; 100];
        let reconstructed = vec![0.5; 100];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0)
            .await
            .unwrap();
        assert!(psnr_val.is_finite());
        assert!(psnr_val < 10.0); // Poor quality
    }

    #[tokio::test]
    async fn test_psnr_boundary() {
        let dev = get_test_device().await;

        // Very small difference (high PSNR)
        let original = vec![0.5; 1000];
        let mut reconstructed = vec![0.5; 1000];
        reconstructed[0] = 0.501; // Tiny difference
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0)
            .await
            .unwrap();
        assert!(psnr_val > 50.0); // High quality

        // Different max pixel value
        let original = vec![128.0; 100];
        let reconstructed = vec![127.0; 100];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 255.0)
            .await
            .unwrap();
        assert!(psnr_val.is_finite());
    }

    #[tokio::test]
    async fn test_psnr_large_batch() {
        let dev = get_test_device().await;

        // 10000 pixels
        let original: Vec<f32> = (0..10000).map(|i| (i % 256) as f32).collect();
        let mut reconstructed = original.clone();
        // Add small noise
        for i in 0..10000 {
            reconstructed[i] += 0.1;
        }
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 255.0)
            .await
            .unwrap();
        assert!(psnr_val.is_finite());
        assert!(psnr_val > 40.0); // Good quality
    }

    #[tokio::test]
    async fn test_psnr_precision() {
        let dev = get_test_device().await;

        // Known MSE calculation
        // original = [1.0, 1.0], reconstructed = [0.0, 2.0]
        // MSE = ((1-0)^2 + (1-2)^2) / 2 = 2/2 = 1.0
        // PSNR = 10 * log10(1.0^2 / 1.0) = 10 * log10(1.0) = 0 dB
        let original = vec![1.0, 1.0];
        let reconstructed = vec![0.0, 2.0];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0)
            .await
            .unwrap();
        assert!((psnr_val - 0.0).abs() < 0.1);
    }
}
