//! RepeatInterleave - Repeat each element
//!
//! Repeats each element specified number of times.

pub async fn repeat_interleave(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    repeats: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = Vec::with_capacity(input.len() * repeats);
    
    for &val in input {
        for _ in 0..repeats {
            output.push(val);
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
    async fn test_repeat_interleave() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 2).await.unwrap();
        assert_eq!(output, vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
    }
}
