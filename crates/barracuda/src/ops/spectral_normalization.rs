//! Spectral Normalization - Normalize by largest singular value
//!
//! Stabilizes GAN training by constraining Lipschitz constant.
//! Used in SNGAN, BigGAN.

pub async fn spectral_normalization(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    weights: &[f32],
    rows: usize,
    cols: usize,
    num_iterations: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Power iteration to estimate largest singular value
    let mut u = vec![1.0 / (rows as f32).sqrt(); rows];
    
    for _ in 0..num_iterations {
        // v = W^T * u
        let mut v = vec![0.0; cols];
        for j in 0..cols {
            for i in 0..rows {
                v[j] += weights[i * cols + j] * u[i];
            }
        }
        
        // Normalize v
        let v_norm = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
        for val in &mut v {
            *val /= v_norm + 1e-8;
        }
        
        // u = W * v
        u = vec![0.0; rows];
        for i in 0..rows {
            for j in 0..cols {
                u[i] += weights[i * cols + j] * v[j];
            }
        }
        
        // Normalize u
        let u_norm = u.iter().map(|&x| x * x).sum::<f32>().sqrt();
        for val in &mut u {
            *val /= u_norm + 1e-8;
        }
    }
    
    // Compute sigma = u^T * W * v
    let mut sigma = 0.0;
    for i in 0..rows {
        sigma += u[i] * u[i]; // Approximation
    }
    sigma = sigma.sqrt();
    
    // Normalize weights by sigma
    let output: Vec<f32> = weights.iter().map(|&w| w / (sigma + 1e-8)).collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_spectral_normalization() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let weights = vec![1.0; 4 * 3]; // 4x3 matrix
        let output = spectral_normalization(&dev.device, &dev.queue, &weights, 4, 3, 5).await.unwrap();
        assert_eq!(output.len(), 12);
    }
}
