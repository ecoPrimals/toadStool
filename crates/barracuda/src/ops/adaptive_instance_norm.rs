//! Adaptive Instance Normalization (AdaIN) - Style transfer
//!
//! Transfers style from one image to another.
//! Used in neural style transfer, GANs.

pub async fn adaptive_instance_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    content: &[f32],
    style_mean: &[f32],   // [channels]
    style_std: &[f32],    // [channels]
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let spatial_size = height * width;
    let mut output = vec![0.0f32; content.len()];
    
    for b in 0..batch_size {
        for c in 0..channels {
            // Compute content statistics
            let mut content_mean = 0.0;
            let mut content_var = 0.0;
            
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                content_mean += content[idx];
            }
            content_mean /= spatial_size as f32;
            
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let diff = content[idx] - content_mean;
                content_var += diff * diff;
            }
            content_var /= spatial_size as f32;
            let content_std = content_var.sqrt();
            
            // Apply AdaIN: normalize content, then scale/shift to style
            for s in 0..spatial_size {
                let idx = b * channels * spatial_size + c * spatial_size + s;
                let normalized = (content[idx] - content_mean) / (content_std + 1e-5);
                output[idx] = normalized * style_std[c] + style_mean[c];
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
    async fn test_adaptive_instance_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let content = vec![1.0; 1 * 3 * 4 * 4];
        let style_mean = vec![0.5, 0.5, 0.5];
        let style_std = vec![0.2, 0.2, 0.2];
        let output = adaptive_instance_norm(&dev.device, &dev.queue, &content, &style_mean, &style_std, 1, 3, 4, 4).await.unwrap();
        assert_eq!(output.len(), content.len());
    }
}
