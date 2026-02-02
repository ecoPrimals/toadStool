//! Take - Advanced indexing operation
//!
//! Gathers elements from input using indices.

pub async fn take(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = indices
        .iter()
        .map(|&idx| {
            if idx < input.len() {
                input[idx]
            } else {
                0.0 // Out of bounds returns 0
            }
        })
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
    async fn test_take_basic() {
        let dev = get_test_device().await;
        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let indices = vec![0, 2, 4];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_take_edge_cases() {
        let dev = get_test_device().await;

        // Single index
        let input = vec![1.0, 2.0, 3.0];
        let indices = vec![1];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 1);

        // Empty indices
        let input = vec![1.0, 2.0];
        let indices: Vec<usize> = vec![];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 0);

        // Out of bounds (returns 0)
        let input = vec![1.0, 2.0];
        let indices = vec![10];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output[0], 0.0);
    }

    #[tokio::test]
    async fn test_take_boundary() {
        let dev = get_test_device().await;

        // Duplicate indices
        let input = vec![10.0, 20.0, 30.0];
        let indices = vec![1, 1, 1];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|&x| x == 20.0));

        // Reverse order
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let indices = vec![3, 2, 1, 0];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 4);
    }

    #[tokio::test]
    async fn test_take_large_batch() {
        let dev = get_test_device().await;

        // Large input
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let indices: Vec<usize> = (0..500).map(|i| i * 2).collect();
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();
        assert_eq!(output.len(), 500);
    }

    #[tokio::test]
    async fn test_take_precision() {
        let dev = get_test_device().await;

        // Verify exact values
        let input = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let indices = vec![0, 2, 4];
        let output = take(&dev.device, &dev.queue, &input, &indices)
            .await
            .unwrap();

        assert_eq!(output[0], 10.0);
        assert_eq!(output[1], 30.0);
        assert_eq!(output[2], 50.0);
    }
}
