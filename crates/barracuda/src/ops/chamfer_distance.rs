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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_chamfer_distance_basic() {
        let dev = get_test_device().await;
        let points1 = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // 2 points
        let points2 = vec![0.1, 0.1, 0.1, 0.9, 0.1, 0.1]; // 2 points
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 2, 2).await.unwrap();
        assert!(dist > 0.0);
        assert!(dist.is_finite());
    }

    #[tokio::test]
    async fn test_chamfer_distance_edge_cases() {
        let dev = get_test_device().await;
        
        // Identical point clouds (zero distance)
        let points1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let points2 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 2, 2).await.unwrap();
        assert!(dist.abs() < 1e-6);
        
        // Single point
        let points1 = vec![0.0, 0.0, 0.0];
        let points2 = vec![1.0, 1.0, 1.0];
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 1, 1).await.unwrap();
        assert!(dist > 0.0);
    }

    #[tokio::test]
    async fn test_chamfer_distance_boundary() {
        let dev = get_test_device().await;
        
        // Asymmetric point clouds (different sizes)
        let points1 = vec![0.0, 0.0, 0.0];
        let points2 = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 1, 3).await.unwrap();
        assert!(dist > 0.0);
        assert!(dist.is_finite());
    }

    #[tokio::test]
    async fn test_chamfer_distance_large_batch() {
        let dev = get_test_device().await;
        
        // Larger point clouds
        let n = 50;
        let m = 60;
        
        let points1: Vec<f32> = (0..n * 3).map(|i| (i % 10) as f32 * 0.1).collect();
        let points2: Vec<f32> = (0..m * 3).map(|i| (i % 12) as f32 * 0.1).collect();
        
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, n, m).await.unwrap();
        assert!(dist >= 0.0);
        assert!(dist.is_finite());
    }

    #[tokio::test]
    async fn test_chamfer_distance_precision() {
        let dev = get_test_device().await;
        
        // Test with known geometry
        let points1 = vec![
            0.0, 0.0, 0.0,  // Origin
            1.0, 0.0, 0.0,  // Unit x
        ];
        let points2 = vec![
            0.0, 0.0, 0.0,  // Origin (matches)
            0.0, 1.0, 0.0,  // Unit y
        ];
        
        let dist = chamfer_distance(&dev.device, &dev.queue, &points1, &points2, 2, 2).await.unwrap();
        
        // Forward: origin→origin (0) + unitX→origin (1) = avg 0.5
        // Backward: origin→origin (0) + unitY→origin (1) = avg 0.5
        // Total = 1.0
        assert!((dist - 1.0).abs() < 0.01);
    }
}
