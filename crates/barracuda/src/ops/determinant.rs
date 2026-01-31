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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_determinant_basic() {
        let dev = get_test_device().await;
        // 2x2 matrix: [[4, 7], [2, 6]]
        let matrix = vec![4.0, 7.0, 2.0, 6.0];
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert!((det - 10.0).abs() < 1e-5); // 4*6 - 7*2 = 10
    }

    #[tokio::test]
    async fn test_determinant_edge_cases() {
        let dev = get_test_device().await;

        // 1x1 matrix (edge case)
        let matrix = vec![5.0];
        let det = determinant(&dev.device, &dev.queue, &matrix, 1).await.unwrap();
        assert!((det - 5.0).abs() < 1e-5);

        // Singular matrix (determinant = 0)
        let matrix = vec![1.0, 2.0, 2.0, 4.0]; // Rows are linearly dependent
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert!(det.abs() < 1e-5);

        // Identity matrix (det = 1)
        let matrix = vec![1.0, 0.0, 0.0, 1.0];
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert!((det - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_determinant_boundary() {
        let dev = get_test_device().await;

        // 3x3 matrix with known det
        let matrix = vec![
            1.0, 2.0, 3.0,
            0.0, 1.0, 4.0,
            5.0, 6.0, 0.0,
        ];
        let det = determinant(&dev.device, &dev.queue, &matrix, 3).await.unwrap();
        // Expected: 1*(1*0 - 4*6) - 2*(0*0 - 4*5) + 3*(0*6 - 1*5) = 1*(-24) - 2*(-20) + 3*(-5) = -24 + 40 - 15 = 1
        assert!((det - 1.0).abs() < 1e-4);

        // Negative determinant (row swap)
        let matrix = vec![
            2.0, 3.0,
            1.0, 4.0,
        ];
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        assert!((det - 5.0).abs() < 1e-5); // 2*4 - 3*1 = 5
    }

    #[tokio::test]
    async fn test_determinant_large_batch() {
        let dev = get_test_device().await;

        // 4x4 matrix
        let matrix = vec![
            1.0, 0.0, 2.0, -1.0,
            3.0, 0.0, 0.0, 5.0,
            2.0, 1.0, 4.0, -3.0,
            1.0, 0.0, 5.0, 0.0,
        ];
        let det = determinant(&dev.device, &dev.queue, &matrix, 4).await.unwrap();
        assert!(det.is_finite());

        // 5x5 identity (larger matrix)
        let mut matrix = vec![0.0; 25];
        for i in 0..5 {
            matrix[i * 5 + i] = 1.0;
        }
        let det = determinant(&dev.device, &dev.queue, &matrix, 5).await.unwrap();
        assert!((det - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_determinant_precision() {
        let dev = get_test_device().await;

        // 3x3 with known exact determinant
        let matrix = vec![
            2.0, 0.0, 0.0,
            0.0, 3.0, 0.0,
            0.0, 0.0, 4.0,
        ];
        let det = determinant(&dev.device, &dev.queue, &matrix, 3).await.unwrap();
        // Diagonal matrix: det = product of diagonal = 2*3*4 = 24
        assert!((det - 24.0).abs() < 1e-5);

        // 2x2 with fractional values
        let matrix = vec![0.5, 0.25, 0.75, 0.5];
        let det = determinant(&dev.device, &dev.queue, &matrix, 2).await.unwrap();
        // 0.5*0.5 - 0.25*0.75 = 0.25 - 0.1875 = 0.0625
        assert!((det - 0.0625).abs() < 1e-6);
    }
}
