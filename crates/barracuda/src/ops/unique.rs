//! Unique - Find unique elements in tensor
//!
//! Returns sorted unique values.

pub async fn unique(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut sorted = input.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let mut unique_vals = Vec::new();
    let mut prev = f32::NAN;
    
    for &val in &sorted {
        if val != prev {
            unique_vals.push(val);
            prev = val;
        }
    }
    
    Ok(unique_vals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_unique() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 1.0, 3.0, 2.0, 3.0, 1.0];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0]);
    }
}
