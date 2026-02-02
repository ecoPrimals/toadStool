//! Diag - Extract or construct diagonal
//!
//! Extract diagonal from matrix or create diagonal matrix from vector.

pub enum DiagMode {
    Extract,   // Matrix -> vector
    Construct, // Vector -> matrix
}

pub async fn diag(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    mode: DiagMode,
    size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    match mode {
        DiagMode::Extract => {
            // Extract diagonal from square matrix
            if input.len() != size * size {
                return Err("Input must be square matrix".into());
            }

            let mut diag_vals = Vec::with_capacity(size);
            for i in 0..size {
                diag_vals.push(input[i * size + i]);
            }
            Ok(diag_vals)
        }
        DiagMode::Construct => {
            // Create diagonal matrix from vector
            if input.len() != size {
                return Err("Input must be vector of size".into());
            }

            let mut matrix = vec![0.0f32; size * size];
            for i in 0..size {
                matrix[i * size + i] = input[i];
            }
            Ok(matrix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_diag_extract_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![1.0, 2.0, 3.0, 4.0];
        let diag_vals = diag(&dev.device, &dev.queue, &matrix, DiagMode::Extract, 2)
            .await
            .unwrap();
        assert_eq!(diag_vals, vec![1.0, 4.0]);
    }

    #[tokio::test]
    async fn test_diag_construct_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let vec = vec![5.0, 6.0];
        let matrix = diag(&dev.device, &dev.queue, &vec, DiagMode::Construct, 2)
            .await
            .unwrap();
        assert_eq!(matrix, vec![5.0, 0.0, 0.0, 6.0]);
    }

    #[tokio::test]
    async fn test_diag_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // 1x1 matrix
        let matrix_1x1 = vec![42.0];
        let diag_1x1 = diag(&dev.device, &dev.queue, &matrix_1x1, DiagMode::Extract, 1)
            .await
            .unwrap();
        assert_eq!(diag_1x1, vec![42.0]);

        // Construct 1x1
        let vec_1 = vec![99.0];
        let matrix_1 = diag(&dev.device, &dev.queue, &vec_1, DiagMode::Construct, 1)
            .await
            .unwrap();
        assert_eq!(matrix_1, vec![99.0]);

        // Extract and reconstruct should be inverse
        let original_diag = vec![1.0, 2.0, 3.0];
        let matrix = diag(
            &dev.device,
            &dev.queue,
            &original_diag,
            DiagMode::Construct,
            3,
        )
        .await
        .unwrap();
        let extracted = diag(&dev.device, &dev.queue, &matrix, DiagMode::Extract, 3)
            .await
            .unwrap();
        assert_eq!(extracted, original_diag);
    }

    #[tokio::test]
    async fn test_diag_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Large diagonal
        let size = 100;
        let diag_vals: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let matrix = diag(
            &dev.device,
            &dev.queue,
            &diag_vals,
            DiagMode::Construct,
            size,
        )
        .await
        .unwrap();

        assert_eq!(matrix.len(), size * size);

        // Verify diagonal values
        for i in 0..size {
            assert_eq!(matrix[i * size + i], i as f32);
        }

        // Verify off-diagonal is zero
        for i in 0..size {
            for j in 0..size {
                if i != j {
                    assert_eq!(matrix[i * size + j], 0.0);
                }
            }
        }

        // Extract back
        let extracted = diag(&dev.device, &dev.queue, &matrix, DiagMode::Extract, size)
            .await
            .unwrap();
        assert_eq!(extracted, diag_vals);
    }

    #[tokio::test]
    async fn test_diag_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test with fractional values
        let precise_diag = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let matrix = diag(
            &dev.device,
            &dev.queue,
            &precise_diag,
            DiagMode::Construct,
            5,
        )
        .await
        .unwrap();

        assert_eq!(matrix.len(), 25);

        // Check precision of diagonal
        for (i, &val) in precise_diag.iter().enumerate() {
            assert!(
                (matrix[i * 5 + i] - val).abs() < 1e-6,
                "Diagonal value {} mismatch",
                i
            );
        }

        // Extract and verify precision maintained
        let extracted = diag(&dev.device, &dev.queue, &matrix, DiagMode::Extract, 5)
            .await
            .unwrap();
        for (i, (&orig, &ext)) in precise_diag.iter().zip(extracted.iter()).enumerate() {
            assert!(
                (orig - ext).abs() < 1e-6,
                "Precision loss at index {}: {} vs {}",
                i,
                orig,
                ext
            );
        }
    }
}
