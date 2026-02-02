//! RepeatInterleave - Repeat each element
//!
//! Repeats each element specified number of times.

pub async fn repeat_interleave(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    repeats: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = Vec::with_capacity(input.len() * repeats);

    for &val in input {
        for _ in 0..repeats {
            output.push(val);
        }
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
    async fn test_repeat_interleave_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 2)
            .await
            .unwrap();
        assert_eq!(output, vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
    }

    #[tokio::test]
    async fn test_repeat_interleave_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let input = vec![5.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 3)
            .await
            .unwrap();
        assert_eq!(output, vec![5.0, 5.0, 5.0]);

        // Repeat once (identity)
        let input = vec![1.0, 2.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 1)
            .await
            .unwrap();
        assert_eq!(output, vec![1.0, 2.0]);
    }

    #[tokio::test]
    async fn test_repeat_interleave_boundary() {
        let dev = get_test_device().await;

        // Large repeat count
        let input = vec![1.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 100)
            .await
            .unwrap();
        assert_eq!(output.len(), 100);
        assert!(output.iter().all(|&x| x == 1.0));

        // Many elements
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 3)
            .await
            .unwrap();
        assert_eq!(output.len(), 30);
    }

    #[tokio::test]
    async fn test_repeat_interleave_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements, repeat 5 times
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 5)
            .await
            .unwrap();
        assert_eq!(output.len(), 5000);
    }

    #[tokio::test]
    async fn test_repeat_interleave_precision() {
        let dev = get_test_device().await;

        // Verify interleaving pattern
        let input = vec![10.0, 20.0, 30.0];
        let output = repeat_interleave(&dev.device, &dev.queue, &input, 2)
            .await
            .unwrap();

        assert_eq!(output[0], 10.0);
        assert_eq!(output[1], 10.0);
        assert_eq!(output[2], 20.0);
        assert_eq!(output[3], 20.0);
        assert_eq!(output[4], 30.0);
        assert_eq!(output[5], 30.0);
    }
}
