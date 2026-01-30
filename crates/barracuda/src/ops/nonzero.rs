//! NonZero - Find indices of non-zero elements
//!
//! Returns indices where values are non-zero.

pub async fn nonzero(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let indices: Vec<usize> = input.iter().enumerate()
        .filter_map(|(idx, &val)| if val != 0.0 { Some(idx) } else { None })
        .collect();
    
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_nonzero() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![0.0, 1.0, 0.0, 2.0, 0.0, 3.0];
        let indices = nonzero(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(indices, vec![1, 3, 5]);
    }
}
