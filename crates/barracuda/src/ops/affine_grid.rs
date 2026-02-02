//! Affine Grid - Generate sampling grid for spatial transformers
//!
//! Creates grid of normalized coordinates based on affine transformation.

pub async fn affine_grid(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    theta: &[f32], // [batch, 2, 3] affine transformation matrix
    batch_size: usize,
    height: usize,
    width: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if theta.len() != batch_size * 2 * 3 {
        return Err("Theta must be [batch, 2, 3]".into());
    }

    let mut grid = vec![0.0f32; batch_size * height * width * 2];

    for b in 0..batch_size {
        for h in 0..height {
            for w in 0..width {
                // Normalized coordinates [-1, 1]
                let x = 2.0 * w as f32 / (width - 1) as f32 - 1.0;
                let y = 2.0 * h as f32 / (height - 1) as f32 - 1.0;

                // Apply affine transform: [x', y'] = theta * [x, y, 1]
                let theta_idx = b * 6;
                let x_prime =
                    theta[theta_idx] * x + theta[theta_idx + 1] * y + theta[theta_idx + 2];
                let y_prime =
                    theta[theta_idx + 3] * x + theta[theta_idx + 4] * y + theta[theta_idx + 5];

                let grid_idx = b * height * width * 2 + h * width * 2 + w * 2;
                grid[grid_idx] = x_prime;
                grid[grid_idx + 1] = y_prime;
            }
        }
    }

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_affine_grid_basic() {
        let dev = get_test_device().await;
        let theta = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]; // Identity
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 4, 4)
            .await
            .unwrap();
        assert_eq!(grid.len(), 1 * 4 * 4 * 2);
        assert!(grid.iter().all(|&x| x.is_finite()));
        // Identity transform should preserve normalized coordinates
        assert!(grid.iter().all(|&x| x >= -1.0 && x <= 1.0));
    }

    #[tokio::test]
    async fn test_affine_grid_edge_cases() {
        let dev = get_test_device().await;

        // Test with translation
        let theta = vec![1.0, 0.0, 0.5, 0.0, 1.0, 0.5]; // Translate by (0.5, 0.5)
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 2, 2)
            .await
            .unwrap();
        assert_eq!(grid.len(), 2 * 2 * 2);
        assert!(grid.iter().all(|&x| x.is_finite()));

        // Test with single pixel
        let theta = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 1, 1)
            .await
            .unwrap();
        assert_eq!(grid.len(), 2);
    }

    #[tokio::test]
    async fn test_affine_grid_boundary() {
        let dev = get_test_device().await;

        // Test with rotation (90 degrees)
        let theta = vec![0.0, -1.0, 0.0, 1.0, 0.0, 0.0];
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 4, 4)
            .await
            .unwrap();
        assert!(grid.iter().all(|&x| x.is_finite()));

        // Test with scaling
        let theta = vec![2.0, 0.0, 0.0, 0.0, 2.0, 0.0]; // Scale by 2x
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 4, 4)
            .await
            .unwrap();
        assert!(grid.iter().all(|&x| x.is_finite()));
        // Scaled coordinates should be larger
        assert!(grid.iter().any(|&x| x.abs() > 1.5));
    }

    #[tokio::test]
    async fn test_affine_grid_large_batch() {
        let dev = get_test_device().await;

        // Multiple batches with different transforms
        let batch_size = 4;
        let height = 8;
        let width = 8;

        let mut theta = Vec::new();
        for i in 0..batch_size {
            let scale = 1.0 + (i as f32) * 0.1;
            theta.extend_from_slice(&[scale, 0.0, 0.0, 0.0, scale, 0.0]);
        }

        let grid = affine_grid(&dev.device, &dev.queue, &theta, batch_size, height, width)
            .await
            .unwrap();

        assert_eq!(grid.len(), batch_size * height * width * 2);
        assert!(grid.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_affine_grid_precision() {
        let dev = get_test_device().await;

        // Test with identity transform - corners should be at [-1,-1], [1,-1], [-1,1], [1,1]
        let theta = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 2, 2)
            .await
            .unwrap();

        // Top-left corner: (-1, -1)
        assert!((grid[0] + 1.0).abs() < 1e-6);
        assert!((grid[1] + 1.0).abs() < 1e-6);

        // Top-right corner: (1, -1)
        assert!((grid[2] - 1.0).abs() < 1e-6);
        assert!((grid[3] + 1.0).abs() < 1e-6);

        // Bottom-left corner: (-1, 1)
        assert!((grid[4] + 1.0).abs() < 1e-6);
        assert!((grid[5] - 1.0).abs() < 1e-6);

        // Bottom-right corner: (1, 1)
        assert!((grid[6] - 1.0).abs() < 1e-6);
        assert!((grid[7] - 1.0).abs() < 1e-6);
    }
}
