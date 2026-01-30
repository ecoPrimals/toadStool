//! ChamferDistance - Chamfer distance for point clouds
//!
//! Bidirectional nearest neighbor distance.
//! Used in 3D reconstruction and point cloud generation.

pub async fn chamfer_distance(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    points1: &[f32], // [N, 3] flattened
    points2: &[f32], // [M, 3] flattened
    n: usize,
    m: usize,
) -> Result<f32, Box<dyn std::error::Error>> {
    if points1.len() != n * 3 || points2.len() != m * 3 {
        return Err("Point dimensions mismatch".into());
    }
    
    // Forward direction: for each point in set1, find nearest in set2
    let mut forward_dist = 0.0;
    for i in 0..n {
        let mut min_dist = f32::MAX;
        
        for j in 0..m {
            let dx = points1[i * 3] - points2[j * 3];
            let dy = points1[i * 3 + 1] - points2[j * 3 + 1];
            let dz = points1[i * 3 + 2] - points2[j * 3 + 2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            
            min_dist = min_dist.min(dist_sq);
        }
        
        forward_dist += min_dist;
    }
    forward_dist /= n as f32;
    
    // Backward direction: for each point in set2, find nearest in set1
    let mut backward_dist = 0.0;
    for j in 0..m {
        let mut min_dist = f32::MAX;
        
        for i in 0..n {
            let dx = points2[j * 3] - points1[i * 3];
            let dy = points2[j * 3 + 1] - points1[i * 3 + 1];
            let dz = points2[j * 3 + 2] - points1[i * 3 + 2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            
            min_dist = min_dist.min(dist_sq);
        }
        
        backward_dist += min_dist;
    }
    backward_dist /= m as f32;
    
    // Chamfer distance = forward + backward
    Ok(forward_dist + backward_dist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_chamfer_distance() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let points1 = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // 2 points
        let points2 = vec![0.1, 0.1, 0.1, 0.9, 0.1, 0.1]; // 2 points
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 2, 2).await.unwrap();
        assert!(dist > 0.0);
    }
}
