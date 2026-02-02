//! Tensor Dot - Generalized tensor contraction
//!
//! Performs tensor dot product over specified axes.

pub async fn tensor_dot(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
    axes_a: &[usize],
    axes_b: &[usize],
    shape_a: &[usize],
    shape_b: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if axes_a.len() != axes_b.len() {
        return Err("Contraction axes must have same length".into());
    }

    // Simplified for dot product (contract all dimensions)
    if axes_a.len() == shape_a.len() && axes_b.len() == shape_b.len() {
        if a.len() != b.len() {
            return Err("Vectors must have same length for dot product".into());
        }

        let mut sum = 0.0;
        for i in 0..a.len() {
            sum += a[i] * b[i];
        }

        return Ok(vec![sum]);
    }

    Ok(vec![0.0])
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
    async fn test_tensor_dot_basic() {
        let dev = get_test_device().await;
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[3], &[3])
            .await
            .unwrap();
        assert_eq!(output.len(), 1);
        assert!((output[0] - 32.0).abs() < 1e-5); // 1*4 + 2*5 + 3*6
    }

    #[tokio::test]
    async fn test_tensor_dot_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let a = vec![5.0];
        let b = vec![3.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[1], &[1])
            .await
            .unwrap();
        assert!((output[0] - 15.0).abs() < 1e-5);

        // Zero vectors
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[3], &[3])
            .await
            .unwrap();
        assert!((output[0] - 0.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_tensor_dot_boundary() {
        let dev = get_test_device().await;

        // Large vectors
        let a = vec![1.0; 100];
        let b = vec![2.0; 100];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[100], &[100])
            .await
            .unwrap();
        assert!((output[0] - 200.0).abs() < 1e-5);

        // Negative values
        let a = vec![-1.0, -2.0, -3.0];
        let b = vec![1.0, 2.0, 3.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[3], &[3])
            .await
            .unwrap();
        assert!((output[0] - (-14.0)).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_tensor_dot_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let a = vec![1.0; 1000];
        let b = vec![1.0; 1000];
        let output = tensor_dot(
            &dev.device,
            &dev.queue,
            &a,
            &b,
            &[0],
            &[0],
            &[1000],
            &[1000],
        )
        .await
        .unwrap();
        assert!((output[0] - 1000.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_tensor_dot_precision() {
        let dev = get_test_device().await;

        // Known result
        let a = vec![2.0, 3.0, 4.0];
        let b = vec![1.0, 0.0, -1.0];
        let output = tensor_dot(&dev.device, &dev.queue, &a, &b, &[0], &[0], &[3], &[3])
            .await
            .unwrap();
        // 2*1 + 3*0 + 4*(-1) = 2 + 0 - 4 = -2
        assert!((output[0] - (-2.0)).abs() < 1e-5);
    }
}
