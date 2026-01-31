//! Renorm - Renormalize with max norm constraint
//!
//! Clamps L2 norm to maximum value.

pub async fn renorm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    max_norm: f32,
    dim: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    let dim_size = shape[dim];
    
    let mut output = input.to_vec();
    
    for o in 0..outer {
        for i in 0..inner {
            let mut norm_sq = 0.0;
            for d in 0..dim_size {
                let idx = o * dim_size * inner + d * inner + i;
                norm_sq += input[idx] * input[idx];
            }
            let norm = norm_sq.sqrt();
            
            if norm > max_norm {
                let scale = max_norm / norm;
                for d in 0..dim_size {
                    let idx = o * dim_size * inner + d * inner + i;
                    output[idx] *= scale;
                }
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
    async fn test_renorm_basic() {
        let dev = get_test_device().await;
        let input = vec![3.0, 4.0]; // Norm = 5.0
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 0, &[2]).await.unwrap();
        // Should be clamped to unit norm
        let norm: f32 = output.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_renorm_edge_cases() {
        let dev = get_test_device().await;

        // Already below max_norm (no change)
        let input = vec![0.3, 0.4]; // Norm = 0.5
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 0, &[2]).await.unwrap();
        assert!((output[0] - 0.3).abs() < 1e-5);
        assert!((output[1] - 0.4).abs() < 1e-5);

        // Zero vector
        let input = vec![0.0, 0.0];
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 0, &[2]).await.unwrap();
        assert_eq!(output, vec![0.0, 0.0]);
    }

    #[tokio::test]
    async fn test_renorm_boundary() {
        let dev = get_test_device().await;

        // Large norm clamped to small max_norm
        let input = vec![10.0, 10.0]; // Norm = 14.14
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 0, &[2]).await.unwrap();
        let norm: f32 = output.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);

        // Multi-dimensional
        let input = vec![3.0, 4.0, 6.0, 8.0]; // 2 vectors
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 1, &[2, 2]).await.unwrap();
        assert_eq!(output.len(), 4);
    }

    #[tokio::test]
    async fn test_renorm_large_batch() {
        let dev = get_test_device().await;

        // 100 elements
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let output = renorm(&dev.device, &dev.queue, &input, 10.0, 0, &[100]).await.unwrap();
        assert_eq!(output.len(), 100);
        let norm: f32 = output.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!(norm <= 10.0 + 1e-4);
    }

    #[tokio::test]
    async fn test_renorm_precision() {
        let dev = get_test_device().await;

        // Test exact scaling
        let input = vec![6.0, 8.0]; // Norm = 10.0
        let output = renorm(&dev.device, &dev.queue, &input, 5.0, 0, &[2]).await.unwrap();
        
        // Should be scaled by 0.5
        assert!((output[0] - 3.0).abs() < 1e-5);
        assert!((output[1] - 4.0).abs() < 1e-5);
        
        let norm: f32 = output.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 5.0).abs() < 1e-5);
    }
}
