//! Diag - Extract or construct diagonal
//!
//! Extract diagonal from matrix or create diagonal matrix from vector.

pub enum DiagMode {
    Extract,  // Matrix -> vector
    Construct // Vector -> matrix
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
        },
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
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_diag_extract() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![1.0, 2.0, 3.0, 4.0];
        let diag_vals = diag(&dev.device, &dev.queue, &matrix, DiagMode::Extract, 2).await.unwrap();
        assert_eq!(diag_vals, vec![1.0, 4.0]);
    }
    
    #[tokio::test]
    async fn test_diag_construct() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let vec = vec![5.0, 6.0];
        let matrix = diag(&dev.device, &dev.queue, &vec, DiagMode::Construct, 2).await.unwrap();
        assert_eq!(matrix, vec![5.0, 0.0, 0.0, 6.0]);
    }
}
