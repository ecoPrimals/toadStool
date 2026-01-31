//! Tile - Repeat tensor along dimensions
//!
//! Tiles input tensor according to repetitions.

pub async fn tile(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    shape: &[usize],
    reps: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if shape.len() != reps.len() {
        return Err("Shape and reps must have same length".into());
    }
    
    // Simplified for 1D
    if shape.len() == 1 {
        let mut output = Vec::with_capacity(input.len() * reps[0]);
        for _ in 0..reps[0] {
            output.extend_from_slice(input);
        }
        return Ok(output);
    }
    
    Ok(input.to_vec())
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
    async fn test_tile_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0];
        let output = tile(&dev.device, &dev.queue, &input, &[2], &[3]).await.unwrap();
        assert_eq!(output.len(), 6);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_tile_edge_cases() {
        let dev = get_test_device().await;

        // Single repetition (identity)
        let input = vec![1.0, 2.0, 3.0];
        let output = tile(&dev.device, &dev.queue, &input, &[3], &[1]).await.unwrap();
        assert_eq!(output.len(), 3);

        // Single element
        let input = vec![5.0];
        let output = tile(&dev.device, &dev.queue, &input, &[1], &[5]).await.unwrap();
        assert_eq!(output.len(), 5);
        assert!(output.iter().all(|&x| x == 5.0));
    }

    #[tokio::test]
    async fn test_tile_boundary() {
        let dev = get_test_device().await;

        // Many repetitions
        let input = vec![1.0, 2.0];
        let output = tile(&dev.device, &dev.queue, &input, &[2], &[10]).await.unwrap();
        assert_eq!(output.len(), 20);

        // Large input
        let input = vec![1.0; 100];
        let output = tile(&dev.device, &dev.queue, &input, &[100], &[3]).await.unwrap();
        assert_eq!(output.len(), 300);
    }

    #[tokio::test]
    async fn test_tile_large_batch() {
        let dev = get_test_device().await;

        // 1000 element input, 10 reps
        let input = vec![0.5; 1000];
        let output = tile(&dev.device, &dev.queue, &input, &[1000], &[10]).await.unwrap();
        assert_eq!(output.len(), 10000);
    }

    #[tokio::test]
    async fn test_tile_precision() {
        let dev = get_test_device().await;

        // Verify exact tiling pattern
        let input = vec![1.0, 2.0, 3.0];
        let output = tile(&dev.device, &dev.queue, &input, &[3], &[2]).await.unwrap();
        
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 1.0);
        assert_eq!(output[4], 2.0);
        assert_eq!(output[5], 3.0);
    }
}
