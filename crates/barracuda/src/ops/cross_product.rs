//! Cross Product - Vector cross product in 3D
//!
//! Computes cross product of 3D vectors.

pub async fn cross_product(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if a.len() % 3 != 0 || b.len() % 3 != 0 || a.len() != b.len() {
        return Err("Inputs must be multiples of 3 and same length".into());
    }
    
    let num_vectors = a.len() / 3;
    let mut output = vec![0.0f32; num_vectors * 3];
    
    for i in 0..num_vectors {
        let a_idx = i * 3;
        let b_idx = i * 3;
        let out_idx = i * 3;
        
        output[out_idx] = a[a_idx + 1] * b[b_idx + 2] - a[a_idx + 2] * b[b_idx + 1];
        output[out_idx + 1] = a[a_idx + 2] * b[b_idx] - a[a_idx] * b[b_idx + 2];
        output[out_idx + 2] = a[a_idx] * b[b_idx + 1] - a[a_idx + 1] * b[b_idx];
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_cross_product() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(cross, vec![0.0, 0.0, 1.0]); // i × j = k
    }
}
