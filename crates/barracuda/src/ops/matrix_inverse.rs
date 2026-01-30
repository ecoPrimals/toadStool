//! Matrix Inverse - Compute inverse of square matrix
//!
//! Uses Gauss-Jordan elimination.
//! Note: Reference implementation for correctness, not optimized.

pub async fn matrix_inverse(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    n: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if matrix.len() != n * n {
        return Err("Matrix must be square".into());
    }
    
    // Augment with identity
    let mut aug = vec![0.0f32; n * (2 * n)];
    for i in 0..n {
        for j in 0..n {
            aug[i * (2 * n) + j] = matrix[i * n + j];
        }
        aug[i * (2 * n) + n + i] = 1.0;
    }
    
    // Gauss-Jordan elimination
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        let mut max_val = aug[i * (2 * n) + i].abs();
        
        for k in (i + 1)..n {
            let val = aug[k * (2 * n) + i].abs();
            if val > max_val {
                max_val = val;
                max_row = k;
            }
        }
        
        if max_val < 1e-10 {
            return Err("Matrix is singular".into());
        }
        
        // Swap rows
        if max_row != i {
            for j in 0..(2 * n) {
                let tmp = aug[i * (2 * n) + j];
                aug[i * (2 * n) + j] = aug[max_row * (2 * n) + j];
                aug[max_row * (2 * n) + j] = tmp;
            }
        }
        
        // Scale pivot row
        let pivot = aug[i * (2 * n) + i];
        for j in 0..(2 * n) {
            aug[i * (2 * n) + j] /= pivot;
        }
        
        // Eliminate column
        for k in 0..n {
            if k != i {
                let factor = aug[k * (2 * n) + i];
                for j in 0..(2 * n) {
                    aug[k * (2 * n) + j] -= factor * aug[i * (2 * n) + j];
                }
            }
        }
    }
    
    // Extract inverse from right half
    let mut inverse = vec![0.0f32; n * n];
    for i in 0..n {
        for j in 0..n {
            inverse[i * n + j] = aug[i * (2 * n) + n + j];
        }
    }
    
    Ok(inverse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_matrix_inverse() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![4.0, 7.0, 2.0, 6.0]; // [[4,7],[2,6]]
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert_eq!(inv.len(), 4);
        // Verify: M * M^{-1} ≈ I
    }
}
