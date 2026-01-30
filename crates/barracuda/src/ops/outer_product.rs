//! Outer Product - Tensor product of vectors
//!
//! Creates matrix from two vectors: M[i,j] = a[i] * b[j]

pub async fn outer_product(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; a.len() * b.len()];
    
    for i in 0..a.len() {
        for j in 0..b.len() {
            output[i * b.len() + j] = a[i] * b[j];
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
    async fn test_outer_product() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 4.0); // 1*4
        assert_eq!(output[1], 5.0); // 1*5
    }
}
