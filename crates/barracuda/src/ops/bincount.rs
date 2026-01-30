//! Bincount - Count occurrences of each value
//!
//! Computes histogram for integer-valued tensors.

pub async fn bincount(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[u32],
    num_bins: usize,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let mut counts = vec![0u32; num_bins];
    
    for &val in input {
        if (val as usize) < num_bins {
            counts[val as usize] += 1;
        }
    }
    
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_bincount() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![0, 1, 1, 2, 2, 2, 3];
        let counts = bincount(&dev.device, &dev.queue, &input, 4).await.unwrap();
        assert_eq!(counts, vec![1, 2, 3, 1]);
    }
}
