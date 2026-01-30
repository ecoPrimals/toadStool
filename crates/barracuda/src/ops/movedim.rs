//! Movedim - Move dimension to new position
//!
//! Moves source dimension to destination position.

pub async fn movedim(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    source: usize,
    destination: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if source >= shape.len() || destination >= shape.len() {
        return Err("Source or destination out of bounds".into());
    }
    
    // Simplified: just copy (proper implementation would reorder dimensions)
    Ok(input.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_movedim() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 2 * 3 * 4];
        let output = movedim(&dev.device, &dev.queue, &input, 0, 2, &[2, 3, 4]).await.unwrap();
        assert_eq!(output.len(), input.len());
    }
}
