//! AdaptiveMaxPool1D - 1D adaptive max pooling
//!
//! Pools to fixed output size regardless of input size.

pub async fn adaptive_max_pool1d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    length: usize,
    output_length: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * channels * output_length];
    
    for b in 0..batch_size {
        for c in 0..channels {
            for ol in 0..output_length {
                let start = (ol * length) / output_length;
                let end = ((ol + 1) * length) / output_length;
                
                let mut max_val = f32::NEG_INFINITY;
                
                for l in start..end {
                    let idx = b * channels * length + c * length + l;
                    max_val = max_val.max(input[idx]);
                }
                
                let out_idx = b * channels * output_length + c * output_length + ol;
                output[out_idx] = max_val;
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
    async fn test_adaptive_max_pool1d() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 16];
        let output = adaptive_max_pool1d(&dev.device, &dev.queue, &input, 1, 3, 16, 8).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 8);
    }
}
