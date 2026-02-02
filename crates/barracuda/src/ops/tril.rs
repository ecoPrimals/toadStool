//! Tril - Lower triangular part of matrix
//!
//! Zeros out elements above diagonal.

pub async fn tril(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    rows: usize,
    cols: usize,
    diagonal: isize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = matrix.to_vec();

    for i in 0..rows {
        for j in 0..cols {
            if (j as isize) > (i as isize + diagonal) {
                output[i * cols + j] = 0.0;
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tril() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]; // 3x3
        let output = tril(&dev.device, &dev.queue, &matrix, 3, 3, 0)
            .await
            .unwrap();
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 0.0); // Above diagonal
    }
}
