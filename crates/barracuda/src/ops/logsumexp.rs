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
    
    #[tokio::test]
    async fn test_logsumexp() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = logsumexp(&dev.device, &dev.queue, &input, 0, &[4]).await.unwrap();
        assert_eq!(output.len(), 1);
    }
}
