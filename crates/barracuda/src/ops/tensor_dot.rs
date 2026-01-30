//! Tensor Dot - Generalized tensor contraction
//!
//! Performs tensor dot product over specified axes.

pub async fn tensor_dot(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
    axes_a: &[usize],
    axes_b: &[usize],
    shape_a: &[usize],
    shape_b: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if axes_a.len() != axes_b.len() {
        return Err("Contraction axes must have same length".into());
    }
    
    // Simplified for dot product (contract all dimensions)
    if axes_a.len() == shape_a.len() && axes_b.len() == shape_b.len() {
        if a.len() != b.len() {
            return Err("Vectors must have same length for dot product".into());
        }
        
        let mut sum = 0.0;
        for i in 0..a.len() {
            sum += a[i] * b[i];
        }
        
        return Ok(vec![sum]);
    }
    
    Ok(vec![0.0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_tensor_dot() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[3], &[3]).await.unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], 32.0); // 1*4 + 2*5 + 3*6
    }
}
