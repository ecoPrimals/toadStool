//! Local Response Normalization (LRN) - AlexNet-style normalization
//!
//! Normalizes across nearby channels.
//! Used in AlexNet (historic).

pub async fn local_response_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    size: usize,    // Normalization window size
    alpha: f32,
    beta: f32,
    k: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; input.len()];
    let half_size = size / 2;
    
    for b in 0..batch_size {
        for c in 0..channels {
            let c_start = c.saturating_sub(half_size);
            let c_end = (c + half_size + 1).min(channels);
            
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let x = input[idx];
                
                // Sum of squares in local window
                let mut sum_sq = 0.0;
                for nc in c_start..c_end {
                    let n_idx = b * channels * spatial_size + nc * spatial_size + s;
                    sum_sq += input[n_idx] * input[n_idx];
                }
                
                let denom = (k + alpha * sum_sq).powf(beta);
                output[idx] = x / denom;
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
    async fn test_local_response_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 4 * 4 * 4];
        let output = local_response_norm(&dev.device, &dev.queue, &input, 1, 4, 4, 4, 3, 0.0001, 0.75, 1.0).await.unwrap();
        assert_eq!(output.len(), input.len());
    }
}
