//! Outer Product - Tensor product of vectors
//!
//! Creates matrix from two vectors: M[i,j] = a[i] * b[j]

pub async fn outer_product(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; a.len() * b.len()];
    
    for i in 0..a.len() {
        for j in 0..b.len() {
            output[i * b.len() + j] = a[i] * b[j];
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
    async fn test_outer_product_basic() {
        let dev = get_test_device().await;
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 4.0); // 1*4
        assert_eq!(output[1], 5.0); // 1*5
        assert_eq!(output[2], 8.0); // 2*4
        assert_eq!(output[3], 10.0); // 2*5
    }

    #[tokio::test]
    async fn test_outer_product_edge_cases() {
        let dev = get_test_device().await;

        // Single element vectors
        let a = vec![3.0];
        let b = vec![7.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], 21.0);

        // One with zeros
        let a = vec![0.0, 1.0];
        let b = vec![5.0, 10.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
    }

    #[tokio::test]
    async fn test_outer_product_boundary() {
        let dev = get_test_device().await;

        // Different sizes
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![1.0, 2.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output.len(), 8);

        // Negative values
        let a = vec![-1.0, 2.0];
        let b = vec![3.0, -4.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output[0], -3.0); // -1 * 3
        assert_eq!(output[1], 4.0);  // -1 * -4
    }

    #[tokio::test]
    async fn test_outer_product_large_vectors() {
        let dev = get_test_device().await;

        // Large vectors
        let a: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..50).map(|i| i as f32).collect();
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(output.len(), 5000);
    }

    #[tokio::test]
    async fn test_outer_product_precision() {
        let dev = get_test_device().await;

        // Test matrix structure
        let a = vec![2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let output = outer_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        
        // Result should be:
        // [[2*4, 2*5, 2*6],
        //  [3*4, 3*5, 3*6]]
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 8.0);
        assert_eq!(output[1], 10.0);
        assert_eq!(output[2], 12.0);
        assert_eq!(output[3], 12.0);
        assert_eq!(output[4], 15.0);
        assert_eq!(output[5], 18.0);
    }
}
