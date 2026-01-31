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
    async fn test_unique_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 1.0, 3.0, 2.0, 3.0, 1.0];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0]);
    }
    
    #[tokio::test]
    async fn test_unique_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Very small values
        let input = vec![-1e-6, -1e-6, 1e-10, 1e-10, 0.0];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 3); // -1e-6, 0.0, 1e-10
        
        // Single unique value
        let input = vec![5.0, 5.0, 5.0, 5.0];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output, vec![5.0]);
    }
    
    #[tokio::test]
    async fn test_unique_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Mix of large and small
        let input = vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY, 0.0, 1e10];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        
        assert_eq!(output.len(), 5);
        assert!(output[0].is_infinite() && output[0].is_sign_negative()); // -inf
        assert!(output[4].is_infinite() && output[4].is_sign_positive()); // +inf
    }
    
    #[tokio::test]
    async fn test_unique_large_tensor() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Large tensor with repeating pattern
        let size = 1000;
        let input: Vec<f32> = (0..size).map(|i| (i % 50) as f32).collect();
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        
        // Should have exactly 50 unique values
        assert_eq!(output.len(), 50);
        
        // Should be sorted
        for i in 0..49 {
            assert!(output[i] < output[i + 1]);
        }
    }
    
    #[tokio::test]
    async fn test_unique_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        
        // Test precision with very close values
        let input = vec![1.0, 1.0001, 1.0001, 1.0002, 1.0];
        let output = unique(&dev.device, &dev.queue, &input).await.unwrap();
        
        assert_eq!(output.len(), 3); // 1.0, 1.0001, 1.0002
        assert!((output[0] - 1.0).abs() < 1e-5);
        assert!((output[1] - 1.0001).abs() < 1e-5);
        assert!((output[2] - 1.0002).abs() < 1e-5);
    }
}
