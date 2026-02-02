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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_matrix_inverse_basic() {
        let dev = get_test_device().await;
        let matrix = vec![4.0, 7.0, 2.0, 6.0]; // [[4,7],[2,6]]
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 2)
            .await
            .unwrap();
        assert_eq!(inv.len(), 4);

        // Verify: M * M^{-1} ≈ I
        let prod = vec![
            matrix[0] * inv[0] + matrix[1] * inv[2],
            matrix[0] * inv[1] + matrix[1] * inv[3],
            matrix[2] * inv[0] + matrix[3] * inv[2],
            matrix[2] * inv[1] + matrix[3] * inv[3],
        ];

        assert!((prod[0] - 1.0).abs() < 0.01); // [0,0] ≈ 1
        assert!(prod[1].abs() < 0.01); // [0,1] ≈ 0
        assert!(prod[2].abs() < 0.01); // [1,0] ≈ 0
        assert!((prod[3] - 1.0).abs() < 0.01); // [1,1] ≈ 1
    }

    #[tokio::test]
    async fn test_matrix_inverse_edge_cases() {
        let dev = get_test_device().await;

        // Identity matrix
        let matrix = vec![1.0, 0.0, 0.0, 1.0];
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 2)
            .await
            .unwrap();
        assert!((inv[0] - 1.0).abs() < 0.01);
        assert!(inv[1].abs() < 0.01);
        assert!(inv[2].abs() < 0.01);
        assert!((inv[3] - 1.0).abs() < 0.01);

        // Diagonal matrix
        let matrix = vec![2.0, 0.0, 0.0, 3.0];
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 2)
            .await
            .unwrap();
        assert!((inv[0] - 0.5).abs() < 0.01);
        assert!((inv[3] - 1.0 / 3.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_matrix_inverse_boundary() {
        let dev = get_test_device().await;

        // 3x3 matrix
        let matrix = vec![1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0];
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 3)
            .await
            .unwrap();
        assert_eq!(inv.len(), 9);
        assert!(inv.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_matrix_inverse_large_matrix() {
        let dev = get_test_device().await;

        // 4x4 identity
        let matrix = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 4)
            .await
            .unwrap();
        assert_eq!(inv.len(), 16);

        // Identity inverse is identity
        for i in 0..4 {
            assert!((inv[i * 4 + i] - 1.0).abs() < 0.01);
        }
    }

    #[tokio::test]
    async fn test_matrix_inverse_precision() {
        let dev = get_test_device().await;

        // Known inverse: [[1,2],[3,4]]^{-1} = [[-2,1],[1.5,-0.5]]
        let matrix = vec![1.0, 2.0, 3.0, 4.0];
        let inv = matrix_inverse(&dev.device, &dev.queue, &matrix, 2)
            .await
            .unwrap();

        assert_eq!(inv.len(), 4);
        // Determinant = 1*4 - 2*3 = -2
        // Expected: [[-2, 1], [1.5, -0.5]]
        assert!((inv[0] - (-2.0)).abs() < 0.01);
        assert!((inv[1] - 1.0).abs() < 0.01);
        assert!((inv[2] - 1.5).abs() < 0.01);
        assert!((inv[3] - (-0.5)).abs() < 0.01);
    }
}
