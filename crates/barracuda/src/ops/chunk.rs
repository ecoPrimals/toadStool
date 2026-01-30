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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_chunk_basic() {
        let dev = get_test_device().await;
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let chunks = chunk(&dev.device, &dev.queue, &input, 3, 0, &[10]).await.unwrap();
        
        assert_eq!(chunks.len(), 3);
        // First chunk: [0,1,2,3] (size 4)
        // Second chunk: [4,5,6,7] (size 4)
        // Third chunk: [8,9] (size 2)
        assert_eq!(chunks[0].len(), 4);
        assert_eq!(chunks[1].len(), 4);
        assert_eq!(chunks[2].len(), 2);
    }

    #[tokio::test]
    async fn test_chunk_edge_cases() {
        let dev = get_test_device().await;

        // Single chunk (no splitting)
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let chunks = chunk(&dev.device, &dev.queue, &input, 1, 0, &[4]).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 4);

        // More chunks than elements
        let input: Vec<f32> = vec![1.0, 2.0];
        let chunks = chunk(&dev.device, &dev.queue, &input, 5, 0, &[2]).await.unwrap();
        assert_eq!(chunks.len(), 2); // Only 2 chunks created
    }

    #[tokio::test]
    async fn test_chunk_boundary() {
        let dev = get_test_device().await;

        // Exact division
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let chunks = chunk(&dev.device, &dev.queue, &input, 4, 0, &[12]).await.unwrap();
        assert_eq!(chunks.len(), 4);
        for chunk in &chunks {
            assert_eq!(chunk.len(), 3); // 12/4 = 3
        }

        // Two chunks
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let chunks = chunk(&dev.device, &dev.queue, &input, 2, 0, &[6]).await.unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 3);
        assert_eq!(chunks[1].len(), 3);
    }

    #[tokio::test]
    async fn test_chunk_large_tensor() {
        let dev = get_test_device().await;

        // 1000 elements into 10 chunks
        let input: Vec<f32> = (0..1000).map(|i| (i as f32) * 0.1).collect();
        let chunks = chunk(&dev.device, &dev.queue, &input, 10, 0, &[1000]).await.unwrap();
        
        assert_eq!(chunks.len(), 10);
        for chunk in &chunks {
            assert_eq!(chunk.len(), 100); // 1000/10 = 100
        }
        
        // Verify data integrity
        let mut reconstructed = Vec::new();
        for chunk in chunks {
            reconstructed.extend(chunk);
        }
        for (i, &val) in reconstructed.iter().enumerate() {
            assert!((val - (i as f32 * 0.1)).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_chunk_precision() {
        let dev = get_test_device().await;

        // Test precision preservation through chunking
        let input: Vec<f32> = vec![1.234, 5.678, 9.012, 3.456, 7.890];
        let chunks = chunk(&dev.device, &dev.queue, &input, 2, 0, &[5]).await.unwrap();
        
        assert_eq!(chunks.len(), 2);
        
        // Verify exact values preserved
        let reconstructed: Vec<f32> = chunks.into_iter().flatten().collect();
        for (r, orig) in reconstructed.iter().zip(input.iter()) {
            assert!((r - orig).abs() < 1e-6, "Value mismatch: {} vs {}", r, orig);
        }
    }
}
