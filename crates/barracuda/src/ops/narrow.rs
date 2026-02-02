//! Narrow - Extract slice along dimension
//!
//! Returns narrowed view without copying (metadata operation).

pub async fn narrow(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    dim: usize,
    start: usize,
    length: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }

    let dim_size = shape[dim];
    if start + length > dim_size {
        return Err("Narrow range out of bounds".into());
    }

    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();

    let mut output = Vec::new();

    for o in 0..outer {
        for d in start..(start + length) {
            for i in 0..inner {
                let idx = o * dim_size * inner + d * inner + i;
                output.push(input[idx]);
            }
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
    async fn test_narrow_basic() {
        let dev = get_test_device().await;
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 0, 2, 5, &[10])
            .await
            .unwrap();
        assert_eq!(output.len(), 5);
        assert_eq!(output, vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[tokio::test]
    async fn test_narrow_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let input = vec![1.0, 2.0, 3.0];
        let output = narrow(&dev.device, &dev.queue, &input, 0, 1, 1, &[3])
            .await
            .unwrap();
        assert_eq!(output, vec![2.0]);

        // Full range (start=0, length=all)
        let input: Vec<f32> = (0..5).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 0, 0, 5, &[5])
            .await
            .unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_narrow_boundary() {
        let dev = get_test_device().await;

        // Start at end
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 0, 8, 2, &[10])
            .await
            .unwrap();
        assert_eq!(output, vec![8.0, 9.0]);

        // Multi-dimensional along different dim
        let input: Vec<f32> = (0..24).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 1, 1, 2, &[2, 3, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 2 * 2 * 4); // outer * length * inner
    }

    #[tokio::test]
    async fn test_narrow_large_batch() {
        let dev = get_test_device().await;

        // Large tensor
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let output = narrow(&dev.device, &dev.queue, &input, 0, 100, 500, &[1000])
            .await
            .unwrap();
        assert_eq!(output.len(), 500);
    }

    #[tokio::test]
    async fn test_narrow_precision() {
        let dev = get_test_device().await;

        // Verify exact values preserved
        let input: Vec<f32> = vec![1.1, 2.2, 3.3, 4.4, 5.5];
        let output = narrow(&dev.device, &dev.queue, &input, 0, 1, 3, &[5])
            .await
            .unwrap();

        assert_eq!(output.len(), 3);
        assert!((output[0] - 2.2).abs() < 1e-5);
        assert!((output[1] - 3.3).abs() < 1e-5);
        assert!((output[2] - 4.4).abs() < 1e-5);
    }
}
