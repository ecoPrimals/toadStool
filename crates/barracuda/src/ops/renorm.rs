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
    
    #[tokio::test]
    async fn test_renorm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![3.0, 4.0]; // Norm = 5.0
        let output = renorm(&dev.device, &dev.queue, &input, 1.0, 0, &[2]).await.unwrap();
        // Should be clamped to unit norm
        let norm: f32 = output.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
