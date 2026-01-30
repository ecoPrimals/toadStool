//! Determinant - Compute matrix determinant
//!
//! Uses LU decomposition for efficiency.

pub async fn determinant(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    n: usize,
) -> Result<f32, Box<dyn std::error::Error>> {
    if matrix.len() != n * n {
        return Err("Matrix must be square".into());
    }
    
    // Simple cases
    if n == 1 {
        return Ok(matrix[0]);
    }
    if n == 2 {
        return Ok(matrix[0] * matrix[3] - matrix[1] * matrix[2]);
    }
    
    // LU decomposition
    let mut lu = matrix.to_vec();
    let mut det = 1.0;
    
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        let mut max_val = lu[i * n + i].abs();
        
        for k in (i + 1)..n {
            let val = lu[k * n + i].abs();
            if val > max_val {
                max_val = val;
                max_row = k;
            }
        }
        
        if max_val < 1e-10 {
            return Ok(0.0); // Singular
        }
        
        // Swap rows (affects det sign)
        if max_row != i {
            det *= -1.0;
            for j in 0..n {
                let tmp = lu[i * n + j];
                lu[i * n + j] = lu[max_row * n + j];
                lu[max_row * n + j] = tmp;
            }
        }
        
        det *= lu[i * n + i];
        
        // Eliminate column
        for k in (i + 1)..n {
            let factor = lu[k * n + i] / lu[i * n + i];
            for j in i..n {
                lu[k * n + j] -= factor * lu[i * n + j];
            }
        }
    }
    
    Ok(det)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_determinant() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![4.0, 7.0, 2.0, 6.0];
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert!((det - 10.0).abs() < 1e-5); // 4*6 - 7*2 = 10
    }
}
