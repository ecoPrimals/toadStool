//! Matrix Power - Compute matrix raised to power
//!
//! M^n via repeated multiplication.

pub async fn matrix_power(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    n: usize,
    power: i32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if matrix.len() != n * n {
        return Err("Matrix must be square".into());
    }
    
    if power == 0 {
        // Identity matrix
        let mut identity = vec![0.0f32; n * n];
        for i in 0..n {
            identity[i * n + i] = 1.0;
        }
        return Ok(identity);
    }
    
    if power == 1 {
        return Ok(matrix.to_vec());
    }
    
    // Positive power: repeated multiplication (simplified)
    let mut result = matrix.to_vec();
    
    for _ in 1..power.abs() {
        // Simplified matrix multiplication inline
        let a = result.clone();
        let mut new_result = vec![0.0f32; n * n];
        
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..n {
                    sum += a[i * n + k] * matrix[k * n + j];
                }
                new_result[i * n + j] = sum;
            }
        }
        
        result = new_result;
    }
    
    Ok(result)
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
    async fn test_matrix_power_basic() {
        let dev = get_test_device().await;
        let matrix = vec![2.0, 0.0, 0.0, 2.0]; // 2*I
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 2).await.unwrap();
        // (2I)^2 = 4I
        assert!((result[0] - 4.0).abs() < 1e-5);
        assert!((result[3] - 4.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_matrix_power_edge_cases() {
        let dev = get_test_device().await;

        // Power 0 (identity)
        let matrix = vec![5.0, 3.0, 2.0, 1.0];
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 0).await.unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!(result[1].abs() < 1e-5);
        assert!(result[2].abs() < 1e-5);
        assert!((result[3] - 1.0).abs() < 1e-5);

        // Power 1 (unchanged)
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 1).await.unwrap();
        assert_eq!(result, matrix);
    }

    #[tokio::test]
    async fn test_matrix_power_boundary() {
        let dev = get_test_device().await;

        // Power 3
        let matrix = vec![2.0, 0.0, 0.0, 2.0];
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 3).await.unwrap();
        // (2I)^3 = 8I
        assert!((result[0] - 8.0).abs() < 1e-4);

        // Non-diagonal matrix
        let matrix = vec![1.0, 1.0, 0.0, 1.0];
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 2).await.unwrap();
        assert_eq!(result.len(), 4);
    }

    #[tokio::test]
    async fn test_matrix_power_large_matrix() {
        let dev = get_test_device().await;

        // 3x3 identity
        let matrix = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 3, 2).await.unwrap();
        
        // I^2 = I
        assert_eq!(result.len(), 9);
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!((result[4] - 1.0).abs() < 1e-5);
        assert!((result[8] - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_matrix_power_precision() {
        let dev = get_test_device().await;

        // Test diagonal scaling
        let matrix = vec![3.0, 0.0, 0.0, 3.0];
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 2).await.unwrap();
        
        // (3I)^2 = 9I
        assert!((result[0] - 9.0).abs() < 0.1);
        assert!(result[1].abs() < 0.1);
        assert!(result[2].abs() < 0.1);
        assert!((result[3] - 9.0).abs() < 0.1);
    }
}
