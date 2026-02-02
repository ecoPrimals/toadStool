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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_earth_mover_distance_basic() {
        let dev = get_test_device().await;
        // Identical distributions
        let dist1 = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let dist2 = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd < 1e-5); // Should be ~0 for identical distributions
    }

    #[tokio::test]
    async fn test_earth_mover_distance_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let dist1 = vec![1.0];
        let dist2 = vec![1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd < 1e-5);

        // Uniform distributions
        let dist1 = vec![1.0, 1.0, 1.0, 1.0];
        let dist2 = vec![1.0, 1.0, 1.0, 1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd < 1e-5);
    }

    #[tokio::test]
    async fn test_earth_mover_distance_boundary() {
        let dev = get_test_device().await;

        // Completely different distributions
        let dist1 = vec![10.0, 0.0, 0.0];
        let dist2 = vec![0.0, 0.0, 10.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd > 0.5); // Large distance for opposite distributions

        // Shifted distributions
        let dist1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dist2 = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd.is_finite() && emd > 0.0);
    }

    #[tokio::test]
    async fn test_earth_mover_distance_large_batch() {
        let dev = get_test_device().await;

        // Large distributions
        let size = 1000;
        let dist1: Vec<f32> = (0..size).map(|i| (i as f32).sin().abs()).collect();
        let dist2: Vec<f32> = (0..size).map(|i| (i as f32).cos().abs()).collect();

        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();
        assert!(emd.is_finite());
        assert!(emd >= 0.0); // EMD is always non-negative
    }

    #[tokio::test]
    async fn test_earth_mover_distance_precision() {
        let dev = get_test_device().await;

        // Known EMD: Point mass at opposite ends
        let dist1 = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let dist2 = vec![0.0, 0.0, 0.0, 0.0, 1.0];
        let emd = earth_mover_distance(&dev.device, &dev.queue, &dist1, &dist2)
            .await
            .unwrap();

        // EMD for point masses at distance 4: accumulated CDF differences
        // CDF1: [1, 1, 1, 1, 1], CDF2: [0, 0, 0, 0, 1]
        // EMD = |1-0| + |1-0| + |1-0| + |1-0| + |1-1| = 4
        assert!((emd - 4.0).abs() < 0.1);

        // Symmetry: EMD(A, B) == EMD(B, A)
        let emd_reverse = earth_mover_distance(&dev.device, &dev.queue, &dist2, &dist1)
            .await
            .unwrap();
        assert!((emd - emd_reverse).abs() < 1e-5);
    }
}
