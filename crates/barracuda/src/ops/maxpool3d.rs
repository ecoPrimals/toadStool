//! MaxPool3D - 3D max pooling
//!
//! For video and volumetric data.

pub async fn maxpool3d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    depth: usize,
    height: usize,
    width: usize,
    kernel_size: usize,
    stride: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_d = (depth - kernel_size) / stride + 1;
    let out_h = (height - kernel_size) / stride + 1;
    let out_w = (width - kernel_size) / stride + 1;
    let mut output = vec![0.0f32; batch_size * channels * out_d * out_h * out_w];
    
    for b in 0..batch_size {
        for c in 0..channels {
            for od in 0..out_d {
                for oh in 0..out_h {
                    for ow in 0..out_w {
                        let mut max_val = f32::NEG_INFINITY;
                        
                        for kd in 0..kernel_size {
                            for kh in 0..kernel_size {
                                for kw in 0..kernel_size {
                                    let id = od * stride + kd;
                                    let ih = oh * stride + kh;
                                    let iw = ow * stride + kw;
                                    
                                    let in_idx = b * channels * depth * height * width
                                               + c * depth * height * width
                                               + id * height * width
                                               + ih * width
                                               + iw;
                                    max_val = max_val.max(input[in_idx]);
                                }
                            }
                        }
                        
                        let out_idx = b * channels * out_d * out_h * out_w
                                    + c * out_d * out_h * out_w
                                    + od * out_h * out_w
                                    + oh * out_w
                                    + ow;
                        output[out_idx] = max_val;
                    }
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
    async fn test_maxpool3d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 2 * 4 * 4 * 4];
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 2, 4, 4, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 2 * 2 * 2 * 2);
    }

    #[tokio::test]
    async fn test_maxpool3d_edge_cases() {
        let dev = get_test_device().await;

        // Minimal volume
        let input = vec![1.0; 1 * 1 * 2 * 2 * 2];
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1);

        // Single channel
        let input = vec![2.0; 1 * 1 * 4 * 4 * 4];
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 4, 2, 2).await.unwrap();
        assert!(output.iter().all(|&x| x == 2.0));
    }

    #[tokio::test]
    async fn test_maxpool3d_boundary() {
        let dev = get_test_device().await;

        // Different stride
        let input = vec![1.0; 1 * 3 * 8 * 8 * 8];
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 8, 2, 4).await.unwrap();
        assert!(output.len() > 0);

        // Many channels
        let input = vec![1.0; 1 * 16 * 6 * 6 * 6];
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 16, 6, 6, 6, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 16 * 3 * 3 * 3);
    }

    #[tokio::test]
    async fn test_maxpool3d_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4
        let batch_size = 4;
        let input = vec![1.0; batch_size * 8 * 10 * 10 * 10];
        let output = maxpool3d(&dev.device, &dev.queue, &input, batch_size, 8, 10, 10, 10, 2, 2).await.unwrap();
        assert_eq!(output.len(), batch_size * 8 * 5 * 5 * 5);
    }

    #[tokio::test]
    async fn test_maxpool3d_precision() {
        let dev = get_test_device().await;

        // Test max selection with varying values
        let mut input = vec![0.0; 1 * 1 * 2 * 2 * 2];
        input[0] = 10.0; // This should be the max
        
        let output = maxpool3d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 2, 2, 2).await.unwrap();
        
        assert_eq!(output.len(), 1);
        assert!((output[0] - 10.0).abs() < 0.1);
    }
}
