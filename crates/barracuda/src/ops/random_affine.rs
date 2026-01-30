//! RandomAffine - Random affine transformations
//!
//! Applies random rotation, translation, scale, and shear.
//! Comprehensive geometric augmentation.

pub async fn random_affine(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    degrees: f32,    // Max rotation in degrees
    translate: (f32, f32), // Max translation fraction
    scale: (f32, f32),     // Scale range
    shear: f32,      // Max shear in degrees
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Generate random parameters
    let angle = degrees * (((seed * 1103515245) % 2000) as f32 / 1000.0 - 1.0);
    let tx = translate.0 * width as f32 * (((seed * 22695477) % 2000) as f32 / 1000.0 - 1.0);
    let ty = translate.1 * height as f32 * (((seed * 1664525) % 2000) as f32 / 1000.0 - 1.0);
    let sc = scale.0 + (scale.1 - scale.0) * ((seed * 48271) % 1000) as f32 / 1000.0;
    let sh = shear * (((seed * 69621) % 2000) as f32 / 1000.0 - 1.0);
    
    // Build affine matrix
    let angle_rad = angle * std::f32::consts::PI / 180.0;
    let shear_rad = sh * std::f32::consts::PI / 180.0;
    
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let tan_sh = shear_rad.tan();
    
    let a = sc * cos_a;
    let b = sc * (-sin_a + cos_a * tan_sh);
    let c = tx;
    let d = sc * sin_a;
    let e = sc * (cos_a + sin_a * tan_sh);
    let f = ty;
    
    let mut output = vec![0.0f32; channels * height * width];
    
    // Apply transformation (inverse mapping)
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    
    for c_idx in 0..channels {
        for i in 0..height {
            for j in 0..width {
                let x = j as f32 - cx;
                let y = i as f32 - cy;
                
                // Inverse transform
                let det = a * e - b * d;
                if det.abs() > 1e-8 {
                    let src_x = (e * (x - c) - b * (y - f)) / det + cx;
                    let src_y = (-d * (x - c) + a * (y - f)) / det + cy;
                    
                    // Bilinear interpolation
                    if src_x >= 0.0 && src_x < width as f32 - 1.0 
                    && src_y >= 0.0 && src_y < height as f32 - 1.0 {
                        let x0 = src_x as usize;
                        let y0 = src_y as usize;
                        let dx = src_x - x0 as f32;
                        let dy = src_y - y0 as f32;
                        
                        let v00 = image[c_idx * height * width + y0 * width + x0];
                        let v01 = image[c_idx * height * width + y0 * width + x0 + 1];
                        let v10 = image[c_idx * height * width + (y0 + 1) * width + x0];
                        let v11 = image[c_idx * height * width + (y0 + 1) * width + x0 + 1];
                        
                        let val = v00 * (1.0 - dx) * (1.0 - dy)
                                + v01 * dx * (1.0 - dy)
                                + v10 * (1.0 - dx) * dy
                                + v11 * dx * dy;
                        
                        output[c_idx * height * width + i * width + j] = val;
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
    async fn test_random_affine() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![1.0; 3 * 64 * 64];
        let transformed = random_affine(&dev.device, &dev.queue, &image, 3, 64, 64, 15.0, (0.1, 0.1), (0.9, 1.1), 5.0, 42424).await.unwrap();
        assert_eq!(transformed.len(), image.len());
    }
}
