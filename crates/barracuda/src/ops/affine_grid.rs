//! Affine Grid - Generate sampling grid for spatial transformers
//!
//! Creates grid of normalized coordinates based on affine transformation.

pub async fn affine_grid(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    theta: &[f32],  // [batch, 2, 3] affine transformation matrix
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
                let x_prime = theta[theta_idx] * x + theta[theta_idx + 1] * y + theta[theta_idx + 2];
                let y_prime = theta[theta_idx + 3] * x + theta[theta_idx + 4] * y + theta[theta_idx + 5];
                
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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_affine_grid() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let theta = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]; // Identity
        let grid = affine_grid(&dev.device, &dev.queue, &theta, 1, 4, 4).await.unwrap();
        assert_eq!(grid.len(), 1 * 4 * 4 * 2);
    }
}
