//! Normalize - L2 normalization along dimension
//!
//! Normalizes vectors to unit length.

pub async fn normalize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    dim: usize,
    shape: &[usize],
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simplified: Normalize along last dimension
    let outer: usize = shape[..dim].iter().product();
    let inner: usize = shape[dim + 1..].iter().product();
    let dim_size = shape[dim];
    
    let mut output = vec![0.0f32; input.len()];
    
    for o in 0..outer {
        for i in 0..inner {
            // Compute L2 norm
            let mut norm_sq = 0.0;
            for d in 0..dim_size {
                let idx = o * dim_size * inner + d * inner + i;
                norm_sq += input[idx] * input[idx];
            }
            let norm = (norm_sq.sqrt() + epsilon);
            
            // Normalize
            for d in 0..dim_size {
                let idx = o * dim_size * inner + d * inner + i;
                output[idx] = input[idx] / norm;
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
    async fn test_normalize() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![3.0, 4.0, 1.0, 0.0]; // Two 2D vectors
        let output = normalize(&dev.device, &dev.queue, &input, 1, &[2, 2], 1e-8).await.unwrap();
        assert_eq!(output.len(), 4);
        // First vector [3,4] normalized should be [0.6, 0.8]
        assert!((output[0] - 0.6).abs() < 1e-5);
        assert!((output[1] - 0.8).abs() < 1e-5);
    }
}
