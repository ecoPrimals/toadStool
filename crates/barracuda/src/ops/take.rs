//! Take - Advanced indexing operation
//!
//! Gathers elements from input using indices.

pub async fn take(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = indices.iter().map(|&idx| {
        if idx < input.len() {
            input[idx]
        } else {
            0.0 // Out of bounds returns 0
        }
    }).collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_take() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let indices = vec![0, 2, 4];
        let output = take(&device, &queue, &input, &indices).await.unwrap();
        assert_eq!(output, vec![10.0, 30.0, 50.0]);
    }
}
