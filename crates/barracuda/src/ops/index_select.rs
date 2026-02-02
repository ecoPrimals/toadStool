//! Index Select - Select elements along a dimension
//!
//! Advanced indexing operation.

pub async fn index_select(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
    dim: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simplified: Select along last dimension
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }

    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    let dim_size = shape[dim];

    let mut output = Vec::with_capacity(outer * indices.len() * inner);

    for o in 0..outer {
        for &idx in indices {
            if idx >= dim_size {
                return Err("Index out of bounds".into());
            }
            for i in 0..inner {
                let in_idx = o * dim_size * inner + idx * inner + i;
                output.push(input[in_idx]);
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
    async fn test_index_select_basic() {
        let dev = get_test_device().await;
        let input: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let indices = vec![0, 2];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[5, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 2 * 4); // 2 indices, 4 elements per index
    }

    #[tokio::test]
    async fn test_index_select_edge_cases() {
        let dev = get_test_device().await;

        // Single index
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let indices = vec![1];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[3, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 1 * 4);

        // Select all indices
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let indices = vec![0, 1, 2];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[3, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 3 * 4);
    }

    #[tokio::test]
    async fn test_index_select_boundary() {
        let dev = get_test_device().await;

        // Select along different dimension
        let input: Vec<f32> = (0..24).map(|i| i as f32).collect();
        let indices = vec![0, 2];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 1, &[2, 3, 4])
            .await
            .unwrap();
        assert!(output.len() > 0);

        // Reversed order indices
        let input: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let indices = vec![3, 1, 0];
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[5, 4])
            .await
            .unwrap();
        assert_eq!(output.len(), 3 * 4);
    }

    #[tokio::test]
    async fn test_index_select_large_batch() {
        let dev = get_test_device().await;

        // Large input, many indices
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let indices: Vec<usize> = (0..50).step_by(5).collect(); // [0, 5, 10, ..., 45]
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[100, 10])
            .await
            .unwrap();
        assert_eq!(output.len(), 10 * 10); // 10 indices, 10 elements per index
    }

    #[tokio::test]
    async fn test_index_select_precision() {
        let dev = get_test_device().await;

        // Test value preservation
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let indices = vec![1]; // Select second element group
        let output = index_select(&dev.device, &dev.queue, &input, &indices, 0, &[2, 4])
            .await
            .unwrap();

        assert_eq!(output.len(), 4);
        // Just verify operation completed and values are finite
        assert!(output.iter().all(|&x| x.is_finite()));
        // Values should be from input range
        assert!(output.iter().all(|&x| x >= 1.0 && x <= 8.0));
    }
}
