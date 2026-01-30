//! Chunk - Split tensor into chunks
//!
//! Divides tensor into specified number of chunks along dimension.

pub async fn chunk(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    num_chunks: usize,
    dim: usize,
    shape: &[usize],
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }
    
    let dim_size = shape[dim];
    let chunk_size = (dim_size + num_chunks - 1) / num_chunks; // Ceiling division
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    
    let mut chunks = Vec::new();
    
    for c in 0..num_chunks {
        let start = c * chunk_size;
        let end = (start + chunk_size).min(dim_size);
        
        if start >= dim_size {
            break;
        }
        
        let mut chunk_data = Vec::new();
        
        for o in 0..outer {
            for d in start..end {
                for i in 0..inner {
                    let idx = o * dim_size * inner + d * inner + i;
                    chunk_data.push(input[idx]);
                }
            }
        }
        
        chunks.push(chunk_data);
    }
    
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_chunk() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let chunks = chunk(&dev.device, &dev.queue, &input, 3, 0, &[10]).await.unwrap();
        assert_eq!(chunks.len(), 3);
    }
}
