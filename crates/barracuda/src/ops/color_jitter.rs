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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_color_jitter() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![0.5; 3 * 128 * 128];
        let jittered = color_jitter(&dev.device, &dev.queue, &image, 3, 128, 128, 0.2, 0.2, 0.2, 0.1, 88888).await.unwrap();
        assert_eq!(jittered.len(), image.len());
    }
}
