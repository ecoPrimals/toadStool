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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_roll_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let output = roll(&dev.device, &dev.queue, &input, 2).await.unwrap();
        assert_eq!(output, vec![4.0, 5.0, 1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn test_roll_edge_cases() {
        let dev = get_test_device().await;

        // Roll by 0 (identity)
        let input = vec![1.0, 2.0, 3.0];
        let output = roll(&dev.device, &dev.queue, &input, 0).await.unwrap();
        assert_eq!(output, input);

        // Negative shift
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = roll(&dev.device, &dev.queue, &input, -2).await.unwrap();
        assert_eq!(output, vec![3.0, 4.0, 1.0, 2.0]);

        // Empty input
        let input: Vec<f32> = vec![];
        let output = roll(&dev.device, &dev.queue, &input, 5).await.unwrap();
        assert_eq!(output, vec![]);
    }

    #[tokio::test]
    async fn test_roll_boundary() {
        let dev = get_test_device().await;

        // Roll by length (full cycle)
        let input = vec![1.0, 2.0, 3.0];
        let output = roll(&dev.device, &dev.queue, &input, 3).await.unwrap();
        assert_eq!(output, input);

        // Roll by more than length
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = roll(&dev.device, &dev.queue, &input, 6).await.unwrap();
        // 6 % 4 = 2
        assert_eq!(output, vec![3.0, 4.0, 1.0, 2.0]);

        // Large negative shift
        let input = vec![1.0, 2.0, 3.0];
        let output = roll(&dev.device, &dev.queue, &input, -7).await.unwrap();
        // -7 % 3 = 2
        assert_eq!(output.len(), 3);
    }

    #[tokio::test]
    async fn test_roll_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let output = roll(&dev.device, &dev.queue, &input, 100).await.unwrap();
        assert_eq!(output.len(), 1000);
        assert_eq!(output[0], 900.0); // Element at position 900 rolled to 0
    }

    #[tokio::test]
    async fn test_roll_precision() {
        let dev = get_test_device().await;

        // Verify exact shifting
        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let output = roll(&dev.device, &dev.queue, &input, 2).await.unwrap();
        
        assert_eq!(output[0], 40.0);
        assert_eq!(output[1], 50.0);
        assert_eq!(output[2], 10.0);
        assert_eq!(output[3], 20.0);
        assert_eq!(output[4], 30.0);
    }
}
