//! Roll - Circular shift operation
//!
//! Shifts elements along an axis with wrap-around.

pub async fn roll(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    shift: isize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let n = input.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    
    let shift = ((shift % n as isize) + n as isize) % n as isize;
    let shift = shift as usize;
    
    let mut output = vec![0.0f32; n];
    for i in 0..n {
        let src = (i + n - shift) % n;
        output[i] = input[src];
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_roll() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let output = roll(&device, &queue, &input, 2).await.unwrap();
        assert_eq!(output, vec![4.0, 5.0, 1.0, 2.0, 3.0]);
    }
}
