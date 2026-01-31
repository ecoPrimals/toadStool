//! Cross Product - Vector cross product in 3D
//!
//! Computes cross product of 3D vectors.

pub async fn cross_product(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if a.len() % 3 != 0 || b.len() % 3 != 0 || a.len() != b.len() {
        return Err("Inputs must be multiples of 3 and same length".into());
    }
    
    let num_vectors = a.len() / 3;
    let mut output = vec![0.0f32; num_vectors * 3];
    
    for i in 0..num_vectors {
        let a_idx = i * 3;
        let b_idx = i * 3;
        let out_idx = i * 3;
        
        output[out_idx] = a[a_idx + 1] * b[b_idx + 2] - a[a_idx + 2] * b[b_idx + 1];
        output[out_idx + 1] = a[a_idx + 2] * b[b_idx] - a[a_idx] * b[b_idx + 2];
        output[out_idx + 2] = a[a_idx] * b[b_idx + 1] - a[a_idx + 1] * b[b_idx];
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_cross_product_basic() {
        let dev = get_test_device().await;
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(cross, vec![0.0, 0.0, 1.0]); // i × j = k
    }

    #[tokio::test]
    async fn test_cross_product_edge_cases() {
        let dev = get_test_device().await;
        
        // Parallel vectors (cross product = 0)
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![2.0, 0.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert!(cross.iter().all(|&x| x.abs() < 1e-6));
        
        // Anti-parallel (also = 0)
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert!(cross.iter().all(|&x| x.abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_cross_product_boundary() {
        let dev = get_test_device().await;
        
        // j × i = -k (anti-commutative)
        let a = vec![0.0, 1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(cross, vec![0.0, 0.0, -1.0]);
        
        // k × i = j
        let a = vec![0.0, 0.0, 1.0];
        let b = vec![1.0, 0.0, 0.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        assert_eq!(cross, vec![0.0, 1.0, 0.0]);
    }

    #[tokio::test]
    async fn test_cross_product_large_batch() {
        let dev = get_test_device().await;
        
        // Multiple vectors
        let num_vectors = 100;
        let mut a = Vec::with_capacity(num_vectors * 3);
        let mut b = Vec::with_capacity(num_vectors * 3);
        
        for _i in 0..num_vectors {
            a.extend_from_slice(&[1.0, 0.0, 0.0]);
            b.extend_from_slice(&[0.0, 1.0, 0.0]);
        }
        
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        
        assert_eq!(cross.len(), num_vectors * 3);
        // All should be [0, 0, 1]
        for i in 0..num_vectors {
            assert_eq!(cross[i * 3], 0.0);
            assert_eq!(cross[i * 3 + 1], 0.0);
            assert_eq!(cross[i * 3 + 2], 1.0);
        }
    }

    #[tokio::test]
    async fn test_cross_product_precision() {
        let dev = get_test_device().await;
        
        // Test with known vectors: [1,2,3] × [4,5,6]
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let cross = cross_product(&dev.device, &dev.queue, &a, &b).await.unwrap();
        
        // Expected: [2*6 - 3*5, 3*4 - 1*6, 1*5 - 2*4] = [12-15, 12-6, 5-8] = [-3, 6, -3]
        assert_eq!(cross.len(), 3);
        assert!((cross[0] - (-3.0)).abs() < 1e-5);
        assert!((cross[1] - 6.0).abs() < 1e-5);
        assert!((cross[2] - (-3.0)).abs() < 1e-5);
    }
}
