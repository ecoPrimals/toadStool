//! Filter Response Normalization (FRN) - Normalization without batch dependency
//!
//! Normalizes activations per filter, not per batch.
//! Enables single-sample inference.

pub async fn filter_response_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    gamma: &[f32],  // [channels]
    beta: &[f32],   // [channels]
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; input.len()];
    
    for b in 0..batch_size {
        for c in 0..channels {
            // Compute squared norm for this filter
            let mut sum_sq = 0.0;
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                sum_sq += input[idx] * input[idx];
            }
            let nu = (sum_sq / spatial_size as f32).sqrt();
            
            // Normalize
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let normalized = input[idx] / (nu + epsilon);
                output[idx] = gamma[c] * normalized + beta[c];
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
    async fn test_filter_response_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let gamma = vec![1.0; 3];
        let beta = vec![0.0; 3];
        let output = filter_response_norm(&dev.device, &dev.queue, &input, &gamma, &beta, 1, 3, 4, 4, 1e-5).await.unwrap();
        assert_eq!(output.len(), input.len());
    }
}
