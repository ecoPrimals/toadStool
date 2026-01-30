//! Index Select - Select elements along a dimension
//!
//! Advanced indexing operation.

pub async fn index_select(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
    dim: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simplified: Select along last dimension
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }
    
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    let dim_size = shape[dim];
    
    let mut output = Vec::with_capacity(outer * indices.len() * inner);
    
    for o in 0..outer {
        for &idx in indices {
            if idx >= dim_size {
                return Err("Index out of bounds".into());
            }
            for i in 0..inner {
                let in_idx = o * dim_size * inner + idx * inner + i;
                output.push(input[in_idx]);
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
    async fn test_index_select() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let indices = vec![0, 2];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[5, 4]).await.unwrap();
        assert!(output.len() > 0);
    }
}
