//! Put - Scatter operation with indexing
//!
//! Places values into output tensor at specified indices.

pub async fn put(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    size: usize,
    indices: &[usize],
    values: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if indices.len() != values.len() {
        return Err("Indices and values must have same length".into());
    }

    let mut output = vec![0.0f32; size];

    for (idx, value) in indices.iter().zip(values.iter()) {
        if *idx < size {
            output[*idx] = *value;
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
    async fn test_put_basic() {
        let dev = get_test_device().await;
        let indices = vec![0, 2, 4];
        let values = vec![10.0, 30.0, 50.0];
        let output = put(&dev.device, &dev.queue, 5, &indices, &values)
            .await
            .unwrap();
        assert_eq!(output, vec![10.0, 0.0, 30.0, 0.0, 50.0]);
    }

    #[tokio::test]
    async fn test_put_edge_cases() {
        let dev = get_test_device().await;

        // Single index
        let indices = vec![3];
        let values = vec![99.0];
        let output = put(&dev.device, &dev.queue, 5, &indices, &values)
            .await
            .unwrap();
        assert_eq!(output[3], 99.0);
        assert_eq!(output.iter().filter(|&&x| x == 0.0).count(), 4);

        // Empty indices
        let indices: Vec<usize> = vec![];
        let values: Vec<f32> = vec![];
        let output = put(&dev.device, &dev.queue, 5, &indices, &values)
            .await
            .unwrap();
        assert_eq!(output, vec![0.0; 5]);
    }

    #[tokio::test]
    async fn test_put_boundary() {
        let dev = get_test_device().await;

        // All indices filled
        let indices = vec![0, 1, 2];
        let values = vec![1.0, 2.0, 3.0];
        let output = put(&dev.device, &dev.queue, 3, &indices, &values)
            .await
            .unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0]);

        // Out of bounds indices (should be ignored)
        let indices = vec![0, 10, 2]; // 10 is out of bounds
        let values = vec![1.0, 999.0, 3.0];
        let output = put(&dev.device, &dev.queue, 5, &indices, &values)
            .await
            .unwrap();
        assert_eq!(output[0], 1.0);
        assert_eq!(output[2], 3.0);
    }

    #[tokio::test]
    async fn test_put_large_batch() {
        let dev = get_test_device().await;

        // 500 indices into 1000-element tensor
        let indices: Vec<usize> = (0..500).map(|i| i * 2).collect();
        let values: Vec<f32> = (0..500).map(|i| i as f32).collect();
        let output = put(&dev.device, &dev.queue, 1000, &indices, &values)
            .await
            .unwrap();

        assert_eq!(output.len(), 1000);
        assert_eq!(output[0], 0.0);
        assert_eq!(output[2], 1.0);
        assert_eq!(output[998], 499.0);
    }

    #[tokio::test]
    async fn test_put_precision() {
        let dev = get_test_device().await;

        // Verify exact value placement
        let indices = vec![1, 3, 5];
        let values = vec![1.5, 2.5, 3.5];
        let output = put(&dev.device, &dev.queue, 7, &indices, &values)
            .await
            .unwrap();

        assert!((output[1] - 1.5).abs() < 1e-5);
        assert!((output[3] - 2.5).abs() < 1e-5);
        assert!((output[5] - 3.5).abs() < 1e-5);
        assert_eq!(output[0], 0.0);
        assert_eq!(output[2], 0.0);
    }
}
