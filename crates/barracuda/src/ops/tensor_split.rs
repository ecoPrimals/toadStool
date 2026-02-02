//! TensorSplit - Split at indices along dimension
//!
//! More flexible than chunk - splits at specific indices.

pub async fn tensor_split(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    indices: &[usize],
    dim: usize,
    shape: &[usize],
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if dim >= shape.len() {
        return Err("Dim out of bounds".into());
    }

    let dim_size = shape[dim];
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();

    let mut splits = Vec::new();
    let mut prev = 0;

    for &idx in indices {
        if idx > dim_size {
            return Err("Split index out of bounds".into());
        }

        let mut split_data = Vec::new();

        for o in 0..outer {
            for d in prev..idx {
                for i in 0..inner {
                    let in_idx = o * dim_size * inner + d * inner + i;
                    split_data.push(input[in_idx]);
                }
            }
        }

        if !split_data.is_empty() {
            splits.push(split_data);
        }
        prev = idx;
    }

    // Last split
    if prev < dim_size {
        let mut split_data = Vec::new();
        for o in 0..outer {
            for d in prev..dim_size {
                for i in 0..inner {
                    let in_idx = o * dim_size * inner + d * inner + i;
                    split_data.push(input[in_idx]);
                }
            }
        }
        splits.push(split_data);
    }

    Ok(splits)
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
    async fn test_tensor_split_basic() {
        let dev = get_test_device().await;
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[3, 7], 0, &[10])
            .await
            .unwrap();
        assert_eq!(splits.len(), 3);
    }

    #[tokio::test]
    async fn test_tensor_split_edge_cases() {
        let dev = get_test_device().await;

        // Single split
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[5], 0, &[10])
            .await
            .unwrap();
        assert_eq!(splits.len(), 2);

        // No splits (just copy)
        let input: Vec<f32> = (0..5).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[], 0, &[5])
            .await
            .unwrap();
        assert_eq!(splits.len(), 1);
    }

    #[tokio::test]
    async fn test_tensor_split_boundary() {
        let dev = get_test_device().await;

        // Split at boundaries
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[1, 9], 0, &[10])
            .await
            .unwrap();
        assert_eq!(splits.len(), 3);

        // Many small splits
        let input: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[2, 4, 6, 8], 0, &[10])
            .await
            .unwrap();
        assert_eq!(splits.len(), 5);
    }

    #[tokio::test]
    async fn test_tensor_split_large_batch() {
        let dev = get_test_device().await;

        // Large tensor
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let splits = tensor_split(
            &dev.device,
            &dev.queue,
            &input,
            &[250, 500, 750],
            0,
            &[1000],
        )
        .await
        .unwrap();
        assert_eq!(splits.len(), 4);
    }

    #[tokio::test]
    async fn test_tensor_split_precision() {
        let dev = get_test_device().await;

        // Verify split sizes
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let splits = tensor_split(&dev.device, &dev.queue, &input, &[4, 8], 0, &[12])
            .await
            .unwrap();

        assert_eq!(splits.len(), 3);
        assert_eq!(splits[0].len(), 4); // [0..4]
        assert_eq!(splits[1].len(), 4); // [4..8]
        assert_eq!(splits[2].len(), 4); // [8..12]
    }
}
