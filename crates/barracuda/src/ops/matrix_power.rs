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
    
    #[tokio::test]
    async fn test_matrix_power() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![2.0, 0.0, 0.0, 2.0]; // 2*I
        let result = matrix_power(&dev.device, &dev.queue, &matrix, 2, 2).await.unwrap();
        // (2I)^2 = 4I
        assert!((result[0] - 4.0).abs() < 1e-5);
    }
}
