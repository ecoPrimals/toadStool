//! Trace - Sum of diagonal elements
//!
//! Computes trace of a matrix.

pub async fn trace(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    matrix: &[f32],
    rows: usize,
    cols: usize,
) -> Result<f32, Box<dyn std::error::Error>> {
    if rows != cols {
        return Err("Trace requires square matrix".into());
    }
    
    let mut sum = 0.0;
    for i in 0..rows {
        sum += matrix[i * cols + i];
    }
    
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_trace() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let matrix = vec![1.0, 2.0, 3.0, 4.0]; // [[1,2],[3,4]]
        let tr = trace(&dev.device, &dev.queue, &matrix, 2, 2).await.unwrap();
        assert_eq!(tr, 5.0); // 1 + 4
    }
}
