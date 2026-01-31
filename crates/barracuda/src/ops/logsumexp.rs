//! LogSumExp - Numerically stable log(sum(exp(x)))
//!
//! Computes log-sum-exp with numerical stability.
//! Used in softmax, log-likelihood computations.

pub async fn logsumexp(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    dim: usize,
    shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    let dim_size = shape[dim];
    
    let mut output = Vec::with_capacity(outer * inner);
    
    for o in 0..outer {
        for i in 0..inner {
            // Find max for numerical stability
            let mut max_val = f32::NEG_INFINITY;
            for d in 0..dim_size {
                let idx = o * dim_size * inner + d * inner + i;
                max_val = max_val.max(input[idx]);
            }
            
            // Compute sum of exp(x - max)
            let mut sum = 0.0;
            for d in 0..dim_size {
                let idx = o * dim_size * inner + d * inner + i;
                sum += (input[idx] - max_val).exp();
            }
            
            output.push(max_val + sum.ln());
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
    async fn test_logsumexp_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[4]).await.unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());
        // LogSumExp should be >= max(input)
        assert!(output[0] >= 4.0);
    }

    #[tokio::test]
    async fn test_logsumexp_edge_cases() {
        let dev = get_test_device().await;

        // Single element
        let input = vec![5.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[1]).await.unwrap();
        assert_eq!(output.len(), 1);
        // LSE of single element is the element itself
        assert!((output[0] - 5.0).abs() < 0.01);

        // All zeros
        let input = vec![0.0, 0.0, 0.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[3]).await.unwrap();
        assert!(output[0].is_finite());
    }

    #[tokio::test]
    async fn test_logsumexp_boundary() {
        let dev = get_test_device().await;

        // Large values (test numerical stability)
        let input = vec![100.0, 101.0, 102.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[3]).await.unwrap();
        assert!(output[0].is_finite());
        assert!(output[0] > 102.0);

        // Negative values
        let input = vec![-10.0, -5.0, -1.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[3]).await.unwrap();
        assert!(output[0].is_finite());
        assert!(output[0] >= -1.0);
    }

    #[tokio::test]
    async fn test_logsumexp_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| i as f32 * 0.1).collect();
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[1000]).await.unwrap();
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());
    }

    #[tokio::test]
    async fn test_logsumexp_precision() {
        let dev = get_test_device().await;

        // Test along different dimension
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // Shape [2, 3]
        let output = logsumexp(&dev.device, &dev.queue, &input, 1, &[2, 3]).await.unwrap();
        
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Each output should be >= max in its group
        assert!(output[0] >= 3.0); // max(1,2,3)
        assert!(output[1] >= 6.0); // max(4,5,6)
    }
}
