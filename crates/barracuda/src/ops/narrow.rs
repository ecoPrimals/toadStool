//! Narrow - Extract slice along dimension
//!
//! Returns narrowed view without copying (metadata operation).

pub async fn narrow(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    dim: usize,
    start: usize,
    length: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }
    
    let dim_size = shape[dim];
    if start + length > dim_size {
        return Err("Narrow range out of bounds".into());
    }
    
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    
    let mut output = Vec::new();
    
    for o in 0..outer {
        for d in start..(start + length) {
            for i in 0..inner {
                let idx = o * dim_size * inner + d * inner + i;
                output.push(input[idx]);
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
    async fn test_narrow() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 0, 2, 5, &[10]).await.unwrap();
        assert_eq!(output, vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    }
}
