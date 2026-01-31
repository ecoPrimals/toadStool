//! LpPool2D - Lp norm pooling
//!
//! Generalizes max (p=∞) and average (p=1) pooling.

pub async fn lp_pool2d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    kernel_size: usize,
    stride: usize,
    p: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = (height - kernel_size) / stride + 1;
    let out_w = (width - kernel_size) / stride + 1;
    let mut output = vec![0.0f32; batch_size * channels * out_h * out_w];
    
    for b in 0..batch_size {
        for c in 0..channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let mut sum = 0.0;
                    
                    for kh in 0..kernel_size {
                        for kw in 0..kernel_size {
                            let ih = oh * stride + kh;
                            let iw = ow * stride + kw;
                            
                            let idx = b * channels * height * width + c * height * width + ih * width + iw;
                            sum += input[idx].abs().powf(p);
                        }
                    }
                    
                    let out_idx = b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
                    output[out_idx] = sum.powf(1.0 / p);
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
    async fn test_lp_pool2d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 2, 2, 2.0).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 4 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_lp_pool2d_edge_cases() {
        let dev = get_test_device().await;

        // p=1 (similar to average pooling)
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 2, 2, 1.0).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 2 * 2);
        assert!(output.iter().all(|&x| x > 0.0));

        // Single channel
        let input = vec![2.0; 1 * 1 * 6 * 6];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 1, 6, 6, 3, 3, 2.0).await.unwrap();
        assert!(output.len() > 0);
    }

    #[tokio::test]
    async fn test_lp_pool2d_boundary() {
        let dev = get_test_device().await;

        // Large p (approaches max pooling)
        let input = vec![1.0; 1 * 2 * 8 * 8];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 2, 8, 8, 2, 2, 10.0).await.unwrap();
        assert_eq!(output.len(), 1 * 2 * 4 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));

        // Different stride
        let input = vec![1.0; 1 * 3 * 16 * 16];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 3, 16, 16, 4, 4, 2.0).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 4 * 4);
    }

    #[tokio::test]
    async fn test_lp_pool2d_large_batch() {
        let dev = get_test_device().await;

        // Batch size 8
        let batch_size = 8;
        let input = vec![1.0; batch_size * 16 * 14 * 14];
        let output = lp_pool2d(&dev.device, &dev.queue, &input, batch_size, 16, 14, 14, 2, 2, 2.0).await.unwrap();
        assert_eq!(output.len(), batch_size * 16 * 7 * 7);
    }

    #[tokio::test]
    async fn test_lp_pool2d_precision() {
        let dev = get_test_device().await;

        // Test p=2 (L2 norm pooling)
        let mut input = vec![0.0; 1 * 1 * 4 * 4];
        input[0] = 3.0; input[1] = 4.0; // Top-left 2x2: [3,4; 0,0]
        
        let output = lp_pool2d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 2, 2, 2.0).await.unwrap();
        
        assert_eq!(output.len(), 1 * 1 * 2 * 2);
        // First output: sqrt(3^2 + 4^2 + 0^2 + 0^2) = sqrt(25) = 5.0
        assert!((output[0] - 5.0).abs() < 0.1);
    }
}
