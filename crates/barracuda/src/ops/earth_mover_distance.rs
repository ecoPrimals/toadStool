//! EarthMoverDistance - Earth Mover's Distance (EMD)
//!
//! Optimal transport distance between distributions.
//! Simplified 1D version (can be extended to higher dimensions).

pub async fn earth_mover_distance(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    dist1: &[f32], // Probability distribution 1
    dist2: &[f32], // Probability distribution 2
) -> Result<f32, Box<dyn std::error::Error>> {
    if dist1.len() != dist2.len() {
        return Err("Distributions must have same length".into());
    }
    
    // Normalize distributions
    let sum1: f32 = dist1.iter().sum();
    let sum2: f32 = dist2.iter().sum();
    
    if sum1 < 1e-8 || sum2 < 1e-8 {
        return Err("Distributions must have positive mass".into());
    }
    
    let norm1: Vec<f32> = dist1.iter().map(|&x| x / sum1).collect();
    let norm2: Vec<f32> = dist2.iter().map(|&x| x / sum2).collect();
    
    // Compute EMD using cumulative difference
    // For 1D: EMD = sum of absolute differences of CDFs
    let mut cdf1 = 0.0;
    let mut cdf2 = 0.0;
    let mut emd = 0.0;
    
    for i in 0..norm1.len() {
        cdf1 += norm1[i];
        cdf2 += norm2[i];
        emd += (cdf1 - cdf2).abs();
    }
    
    Ok(emd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_earth_mover_distance() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let dist1 = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let dist2 = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2).await.unwrap();
        assert!(emd < 1e-5); // Should be ~0 for identical distributions
    }
}
