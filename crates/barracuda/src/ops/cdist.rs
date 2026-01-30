//! Cdist - Pairwise distance computation
//!
//! Computes distances between all pairs of samples.
//! Used in k-NN, clustering, metric learning.

pub async fn cdist(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    x1: &[f32],  // [n1, dim]
    x2: &[f32],  // [n2, dim]
    n1: usize,
    n2: usize,
    dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if x1.len() != n1 * dim || x2.len() != n2 * dim {
        return Err("Dimension mismatch".into());
    }
    
    let mut distances = vec![0.0f32; n1 * n2];
    
    for i in 0..n1 {
        for j in 0..n2 {
            let mut dist_sq = 0.0;
            
            for d in 0..dim {
                let diff = x1[i * dim + d] - x2[j * dim + d];
                dist_sq += diff * diff;
            }
            
            distances[i * n2 + j] = dist_sq.sqrt();
        }
    }
    
    Ok(distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_cdist() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let x1 = vec![0.0, 0.0, 1.0, 0.0]; // 2 points in 2D
        let x2 = vec![0.0, 1.0, 1.0, 1.0]; // 2 points in 2D
        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 2, 2, 2).await.unwrap();
        assert_eq!(distances.len(), 4);
    }
}
