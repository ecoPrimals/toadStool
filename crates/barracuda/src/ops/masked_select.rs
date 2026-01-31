//! Masked Select - Extract elements where mask is true
//!
//! Returns only elements where mask is true.

pub async fn masked_select(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    mask: &[bool],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() != mask.len() {
        return Err("Input and mask must have same length".into());
    }
    
    let output: Vec<f32> = input.iter().zip(mask.iter())
        .filter_map(|(&val, &m)| if m { Some(val) } else { None })
        .collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_masked_select_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mask = vec![true, false, true, false, true];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output, vec![1.0, 3.0, 5.0]);
    }

    #[tokio::test]
    async fn test_masked_select_edge_cases() {
        let dev = get_test_device().await;

        // All selected
        let input = vec![1.0, 2.0, 3.0];
        let mask = vec![true, true, true];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0]);

        // None selected
        let input = vec![1.0, 2.0, 3.0];
        let mask = vec![false, false, false];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output.len(), 0);
    }

    #[tokio::test]
    async fn test_masked_select_boundary() {
        let dev = get_test_device().await;

        // Single element selected
        let input = vec![42.0, 99.0];
        let mask = vec![true, false];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output, vec![42.0]);

        // Single element not selected
        let input = vec![42.0];
        let mask = vec![false];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output.len(), 0);
    }

    #[tokio::test]
    async fn test_masked_select_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements, select every 3rd
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let mask: Vec<bool> = (0..1000).map(|i| i % 3 == 0).collect();
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        
        // Should have ~333 elements
        assert!(output.len() >= 333 && output.len() <= 334);
        // Verify values are correct
        for (idx, &val) in output.iter().enumerate() {
            assert_eq!(val, (idx * 3) as f32);
        }
    }

    #[tokio::test]
    async fn test_masked_select_precision() {
        let dev = get_test_device().await;

        // Test with negative values
        let input = vec![-1.0, -2.0, -3.0, 4.0, 5.0];
        let mask = vec![true, false, true, false, true];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], -1.0);
        assert_eq!(output[1], -3.0);
        assert_eq!(output[2], 5.0);
    }
}
