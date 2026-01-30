//! Searchsorted - Find insertion indices in sorted array
//!
//! Binary search to find where values would be inserted.

pub async fn searchsorted(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    sorted_sequence: &[f32],
    values: &[f32],
    side: bool, // true = left, false = right
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let indices: Vec<usize> = values.iter().map(|&val| {
        // Binary search
        let mut left = 0;
        let mut right = sorted_sequence.len();
        
        while left < right {
            let mid = (left + right) / 2;
            let cond = if side {
                sorted_sequence[mid] < val
            } else {
                sorted_sequence[mid] <= val
            };
            
            if cond {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        left
    }).collect();
    
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_searchsorted() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let sorted = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let values = vec![0.0, 4.0, 10.0];
        let indices = searchsorted(&dev.device, &dev.queue, &sorted, &values, true).await.unwrap();
        assert_eq!(indices, vec![0, 2, 5]);
    }
}
