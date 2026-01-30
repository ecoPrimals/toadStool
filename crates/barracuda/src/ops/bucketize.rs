//! Bucketize - Assign elements to bins
//!
//! Maps each input value to its bin index.

pub async fn bucketize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    boundaries: &[f32], // Must be sorted
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let output: Vec<usize> = input.iter().map(|&val| {
        // Binary search for bin
        boundaries.iter().take_while(|&&b| val >= b).count()
    }).collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_bucketize() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![0.5, 1.5, 2.5, 3.5];
        let boundaries = vec![1.0, 2.0, 3.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries).await.unwrap();
        assert_eq!(bins, vec![0, 1, 2, 3]);
    }
}
