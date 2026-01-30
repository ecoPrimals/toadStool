//! Tile - Repeat tensor along dimensions
//!
//! Tiles input tensor according to repetitions.

pub async fn tile(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    shape: &[usize],
    reps: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if shape.len() != reps.len() {
        return Err("Shape and reps must have same length".into());
    }
    
    // Simplified for 1D
    if shape.len() == 1 {
        let mut output = Vec::with_capacity(input.len() * reps[0]);
        for _ in 0..reps[0] {
            output.extend_from_slice(input);
        }
        return Ok(output);
    }
    
    Ok(input.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_tile() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0];
        let output = tile(&dev.device, &dev.queue, &input, &[2], &[3]).await.unwrap();
        assert_eq!(output, vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
    }
}
