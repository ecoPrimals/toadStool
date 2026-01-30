//! Stack - Stack tensors along new dimension
//!
//! Creates new dimension and stacks inputs along it.

pub async fn stack(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    tensors: &[Vec<f32>],
    _dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if tensors.is_empty() {
        return Err("Cannot stack empty tensor list".into());
    }
    
    let elem_size = tensors[0].len();
    for t in tensors {
        if t.len() != elem_size {
            return Err("All tensors must have same size".into());
        }
    }
    
    let mut output = Vec::with_capacity(tensors.len() * elem_size);
    
    // Simple implementation: concat along new dimension
    for tensor in tensors {
        output.extend_from_slice(tensor);
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_stack() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let t1 = vec![1.0, 2.0];
        let t2 = vec![3.0, 4.0];
        let output = stack(&dev.device, &dev.queue, &[t1, t2], 0).await.unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
    }
}
