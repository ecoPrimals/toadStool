//! TensorSplit - Split at indices along dimension
//!
//! More flexible than chunk - splits at specific indices.

pub async fn tensor_split(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
    dim: usize,
    shape: &[usize],
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }
    
    let dim_size = shape[dim];
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    
    let mut splits = Vec::new();
    let mut prev = 0;
    
    for &idx in indices {
        if idx > dim_size {
            return Err("Split index out of bounds".into());
        }
        
        let mut split_data = Vec::new();
        
        for o in 0..outer {
            for d in prev..idx {
                for i in 0..inner {
                    let in_idx = o * dim_size * inner + d * inner + i;
                    split_data.push(input[in_idx]);
                }
            }
        }
        
        if !split_data.is_empty() {
            splits.push(split_data);
        }
        prev = idx;
    }
    
    // Last split
    if prev < dim_size {
        let mut split_data = Vec::new();
        for o in 0..outer {
            for d in prev..dim_size {
                for i in 0..inner {
                    let in_idx = o * dim_size * inner + d * inner + i;
                    split_data.push(input[in_idx]);
                }
            }
        }
        splits.push(split_data);
    }
    
    Ok(splits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_tensor_split() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[3, 7], 0, &[10]).await.unwrap();
        assert_eq!(splits.len(), 3);
    }
}
