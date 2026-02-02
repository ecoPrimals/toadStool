//! Cdist - Pairwise distance computation
//!
//! Computes distances between all pairs of samples.
//! Used in k-NN, clustering, metric learning.

pub async fn cdist(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    x1: &[f32], // [n1, dim]
    x2: &[f32], // [n2, dim]
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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_cdist_basic() {
        let dev = get_test_device().await;
        let x1 = vec![0.0, 0.0, 1.0, 0.0]; // 2 points in 2D
        let x2 = vec![0.0, 1.0, 1.0, 1.0]; // 2 points in 2D
        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 2, 2, 2)
            .await
            .unwrap();
        assert_eq!(distances.len(), 4);
        assert!(distances.iter().all(|&x| x.is_finite()));
        assert!(distances.iter().all(|&x| x >= 0.0));
    }

    #[tokio::test]
    async fn test_cdist_edge_cases() {
        let dev = get_test_device().await;

        // Single point
        let x1 = vec![1.0, 2.0];
        let x2 = vec![3.0, 4.0];
        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 1, 1, 2)
            .await
            .unwrap();
        assert_eq!(distances.len(), 1);
        // Distance between (1,2) and (3,4) = sqrt(4+4) = sqrt(8) ≈ 2.828
        assert!((distances[0] - 2.828).abs() < 0.01);

        // Identical points (zero distance)
        let x1 = vec![1.0, 2.0, 3.0];
        let x2 = vec![1.0, 2.0, 3.0];
        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 1, 1, 3)
            .await
            .unwrap();
        assert!(distances[0].abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_cdist_boundary() {
        let dev = get_test_device().await;

        // One point vs many
        let x1 = vec![0.0, 0.0];
        let x2 = vec![
            1.0, 0.0, // distance 1
            0.0, 1.0, // distance 1
            1.0, 1.0, // distance sqrt(2)
        ];
        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 1, 3, 2)
            .await
            .unwrap();

        assert_eq!(distances.len(), 3);
        assert!((distances[0] - 1.0).abs() < 1e-6);
        assert!((distances[1] - 1.0).abs() < 1e-6);
        assert!((distances[2] - 1.414).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_cdist_large_batch() {
        let dev = get_test_device().await;

        // Many points in high dimensions
        let n1 = 10;
        let n2 = 20;
        let dim = 5;

        let x1: Vec<f32> = (0..n1 * dim).map(|i| (i % 10) as f32).collect();
        let x2: Vec<f32> = (0..n2 * dim).map(|i| (i % 8) as f32).collect();

        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, n1, n2, dim)
            .await
            .unwrap();

        assert_eq!(distances.len(), n1 * n2);
        assert!(distances.iter().all(|&x| x.is_finite()));
        assert!(distances.iter().all(|&x| x >= 0.0));
    }

    #[tokio::test]
    async fn test_cdist_precision() {
        let dev = get_test_device().await;

        // Test with known distances
        let x1 = vec![
            0.0, 0.0, // Origin
            3.0, 4.0, // Point at distance 5 from origin
        ];
        let x2 = vec![
            0.0, 0.0, // Origin
        ];

        let distances = cdist(&dev.device, &dev.queue, &x1, &x2, 2, 1, 2)
            .await
            .unwrap();

        // Distance from origin to origin = 0
        assert!(distances[0].abs() < 1e-6);

        // Distance from (3,4) to origin = 5
        assert!((distances[1] - 5.0).abs() < 1e-6);
    }
}
