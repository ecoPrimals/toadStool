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
    
    #[tokio::test]
    async fn test_psnr() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let original = vec![0.5; 1000];
        let reconstructed = vec![0.5; 1000];
        let psnr_val = psnr(&dev.device, &dev.queue, &original, &reconstructed, 1.0).await.unwrap();
        assert!(psnr_val > 100.0); // Should be very high for identical signals
    }
}
