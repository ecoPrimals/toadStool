//! SSIM - Structural Similarity Index (Wang et al.)
//!
//! Perceptual similarity metric for images.
//! Considers luminance, contrast, and structure.

pub async fn ssim(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image1: &[f32],
    image2: &[f32],
    width: usize,
    height: usize,
    window_size: usize,
    c1: f32, // Stability constant for luminance
    c2: f32, // Stability constant for contrast
) -> Result<f32, Box<dyn std::error::Error>> {
    if image1.len() != image2.len() || image1.len() != width * height {
        return Err("Image dimensions mismatch".into());
    }
    
    let mut ssim_sum = 0.0;
    let mut count = 0;
    
    // Slide window across image
    for i in 0..=(height - window_size) {
        for j in 0..=(width - window_size) {
            let mut sum1 = 0.0;
            let mut sum2 = 0.0;
            let mut sum1_sq = 0.0;
            let mut sum2_sq = 0.0;
            let mut sum12 = 0.0;
            let n = (window_size * window_size) as f32;
            
            // Compute statistics in window
            for wi in 0..window_size {
                for wj in 0..window_size {
                    let idx = (i + wi) * width + (j + wj);
                    let val1 = image1[idx];
                    let val2 = image2[idx];
                    
                    sum1 += val1;
                    sum2 += val2;
                    sum1_sq += val1 * val1;
                    sum2_sq += val2 * val2;
                    sum12 += val1 * val2;
                }
            }
            
            let mean1 = sum1 / n;
            let mean2 = sum2 / n;
            let var1 = sum1_sq / n - mean1 * mean1;
            let var2 = sum2_sq / n - mean2 * mean2;
            let covar = sum12 / n - mean1 * mean2;
            
            // SSIM formula
            let numerator = (2.0 * mean1 * mean2 + c1) * (2.0 * covar + c2);
            let denominator = (mean1 * mean1 + mean2 * mean2 + c1) * (var1 + var2 + c2);
            
            ssim_sum += numerator / denominator;
            count += 1;
        }
    }
    
    Ok(ssim_sum / count as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_ssim() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image1 = vec![0.5; 64 * 64];
        let image2 = vec![0.5; 64 * 64];
        let similarity = ssim(&dev.device, &dev.queue, &image1, &image2, 64, 64, 11, 0.01, 0.03).await.unwrap();
        assert!((similarity - 1.0).abs() < 0.1); // Should be close to 1.0 for identical images
    }
}
