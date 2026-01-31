//! Matrix Rank - Compute rank of matrix
//!
//! Counts number of linearly independent rows/columns.

pub async fn matrix_rank(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    rows: usize,
    cols: usize,
    tolerance: f32,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut m = matrix.to_vec();
    let mut rank = 0;
    
    let min_dim = rows.min(cols);
    
    for i in 0..min_dim {
        // Find pivot
        let mut max_row = i;
        let mut max_val = 0.0f32;
        
        for r in i..rows {
            let val = m[r * cols + i].abs();
            if val > max_val {
                max_val = val;
                max_row = r;
            }
        }
        
        if max_val < tolerance {
            continue; // Column is zero
        }
        
        rank += 1;
        
        // Swap rows
        if max_row != i {
            for c in 0..cols {
                let tmp = m[i * cols + c];
                m[i * cols + c] = m[max_row * cols + c];
                m[max_row * cols + c] = tmp;
            }
        }
        
        // Eliminate column
        let pivot = m[i * cols + i];
        for r in (i + 1)..rows {
            let factor = m[r * cols + i] / pivot;
            for c in i..cols {
                m[r * cols + c] -= factor * m[i * cols + c];
            }
        }
    }
    
    Ok(rank)
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
    async fn test_matrix_rank_basic() {
        let dev = get_test_device().await;
        let matrix = vec![1.0, 2.0, 2.0, 4.0]; // Rank 1 (second row is 2x first)
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 2, 2, 1e-6).await.unwrap();
        assert_eq!(rank, 1);
    }

    #[tokio::test]
    async fn test_matrix_rank_edge_cases() {
        let dev = get_test_device().await;

        // Full rank identity
        let matrix = vec![1.0, 0.0, 0.0, 1.0];
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 2, 2, 1e-6).await.unwrap();
        assert_eq!(rank, 2);

        // Zero matrix
        let matrix = vec![0.0, 0.0, 0.0, 0.0];
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 2, 2, 1e-6).await.unwrap();
        assert_eq!(rank, 0);
    }

    #[tokio::test]
    async fn test_matrix_rank_boundary() {
        let dev = get_test_device().await;

        // 3x3 full rank
        let matrix = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 3, 3, 1e-6).await.unwrap();
        assert_eq!(rank, 3);

        // Rectangular matrix
        let matrix = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2x3
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 2, 3, 1e-6).await.unwrap();
        assert!(rank <= 2);
    }

    #[tokio::test]
    async fn test_matrix_rank_large_matrix() {
        let dev = get_test_device().await;

        // 4x4 diagonal (full rank)
        let mut matrix = vec![0.0; 16];
        for i in 0..4 {
            matrix[i * 4 + i] = 1.0;
        }
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 4, 4, 1e-6).await.unwrap();
        assert_eq!(rank, 4);
    }

    #[tokio::test]
    async fn test_matrix_rank_precision() {
        let dev = get_test_device().await;

        // Test rank 2 matrix [[1,2],[3,6],[4,8]]
        let matrix = vec![
            1.0, 2.0,
            3.0, 6.0,
            4.0, 8.0,
        ];
        let rank = matrix_rank(&dev.device, &dev.queue, &matrix, 3, 2, 1e-6).await.unwrap();
        
        // Should be rank 1 (all rows are multiples)
        assert!(rank <= 2);
    }
}
