//! Flatten - Flatten tensor to 1D or 2D
//!
//! Collapses dimensions.

pub async fn flatten(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    _start_dim: usize,
    _end_dim: usize,
    _shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simple case: just return copy (reshape is metadata operation)
    Ok(input.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_flatten() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 2 * 3 * 4];
        let output = flatten(&dev.device, &dev.queue, &input, 0, 2, &[2, 3, 4]).await.unwrap();
        assert_eq!(output.len(), input.len());
    }
}
