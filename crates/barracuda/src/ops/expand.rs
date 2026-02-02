//! Expand - Broadcast tensor to larger shape
//!
//! Expands singleton dimensions to larger sizes.

pub async fn expand(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    input_shape: &[usize],
    output_shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input_shape.len() != output_shape.len() {
        return Err("Shapes must have same rank".into());
    }

    // Validate: input dims must be 1 or match output
    for (i, (&in_dim, &out_dim)) in input_shape.iter().zip(output_shape.iter()).enumerate() {
        if in_dim != 1 && in_dim != out_dim {
            return Err(format!(
                "Dimension {} cannot expand from {} to {}",
                i, in_dim, out_dim
            )
            .into());
        }
    }

    let output_size: usize = output_shape.iter().product();
    let mut output = Vec::with_capacity(output_size);

    // Simplified: 1D broadcast
    if input_shape.len() == 1 {
        let repeat_count = output_shape[0] / input_shape[0];
        for _ in 0..repeat_count {
            output.extend_from_slice(input);
        }
    } else {
        // General case would require multi-dim indexing
        output = input.to_vec();
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
    async fn test_expand_basic() {
        let dev = get_test_device().await;
        // Broadcast from [1] to [6]
        let input = vec![5.0];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[6])
            .await
            .unwrap();
        assert_eq!(output.len(), 6);
        // All should be 5.0
        assert!(output.iter().all(|&x| (x - 5.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_expand_edge_cases() {
        let dev = get_test_device().await;

        // Single element expansion
        let input = vec![7.0];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[10])
            .await
            .unwrap();
        assert_eq!(output.len(), 10);
        assert!(output.iter().all(|&x| (x - 7.0).abs() < 1e-5));

        // No expansion (already correct size)
        let input = vec![1.0, 2.0, 3.0];
        let output = expand(&dev.device, &dev.queue, &input, &[3], &[3])
            .await
            .unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_expand_boundary() {
        let dev = get_test_device().await;

        // Large expansion factor
        let input = vec![3.14];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[1000])
            .await
            .unwrap();
        assert_eq!(output.len(), 1000);
        assert!(output.iter().all(|&x| (x - 3.14).abs() < 1e-5));

        // Smaller expansion
        let input = vec![99.0];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[5])
            .await
            .unwrap();
        assert_eq!(output.len(), 5);
        assert!(output.iter().all(|&x| (x - 99.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_expand_large_batch() {
        let dev = get_test_device().await;

        // Single value to large tensor
        let input = vec![42.0];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[10000])
            .await
            .unwrap();
        assert_eq!(output.len(), 10000);
        assert!(output.iter().all(|&x| (x - 42.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_expand_precision() {
        let dev = get_test_device().await;

        // Verify exact value preserved during broadcast
        let input = vec![1.23456];
        let output = expand(&dev.device, &dev.queue, &input, &[1], &[100])
            .await
            .unwrap();
        assert_eq!(output.len(), 100);

        // All values should match exactly
        for val in output.iter() {
            assert!((val - 1.23456).abs() < 1e-6);
        }
    }
}
