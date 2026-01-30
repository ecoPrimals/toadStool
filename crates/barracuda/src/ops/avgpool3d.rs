//! AvgPool3D - 3D average pooling
//!
//! For video and volumetric data.

pub async fn avgpool3d(
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
    let pool_size = (kernel_size * kernel_size * kernel_size) as f32;
    
    for b in 0..batch_size {
        for c in 0..channels {
            for od in 0..out_d {
                for oh in 0..out_h {
                    for ow in 0..out_w {
                        let mut sum = 0.0;
                        
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
                                    sum += input[in_idx];
                                }
                            }
                        }
                        
                        let out_idx = b * channels * out_d * out_h * out_w
                                    + c * out_d * out_h * out_w
                                    + od * out_h * out_w
                                    + oh * out_w
                                    + ow;
                        output[out_idx] = sum / pool_size;
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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_avgpool3d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 2 * 4 * 4 * 4];
        let output = avgpool3d(&dev.device, &dev.queue, &input, 1, 2, 4, 4, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 2 * 2 * 2 * 2);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Constant input should produce constant output
        assert!(output.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_avgpool3d_edge_cases() {
        let dev = get_test_device().await;
        
        // Test with kernel_size = stride (non-overlapping)
        let input = vec![1.0; 1 * 1 * 4 * 4 * 4];
        let output = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 2 * 2 * 2);
        assert!(output.iter().all(|&x| (x - 1.0).abs() < 1e-6));
        
        // Test with stride=1 (overlapping)
        let input = vec![1.0; 1 * 1 * 4 * 4 * 4];
        let output = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 4, 2, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 3 * 3 * 3);
    }

    #[tokio::test]
    async fn test_avgpool3d_boundary() {
        let dev = get_test_device().await;
        
        // Test with varying values
        let input: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let output = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 2 * 2 * 2);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Output should be averages of 2×2×2 regions
        assert!(output[0] < output[output.len() - 1]);
        
        // Test with different kernel sizes
        let input = vec![1.0; 1 * 1 * 8 * 8 * 8];
        let output1 = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 8, 8, 8, 2, 2).await.unwrap();
        let output2 = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 8, 8, 8, 4, 4).await.unwrap();
        
        assert!(output1.len() > output2.len()); // Smaller kernel → more output
    }

    #[tokio::test]
    async fn test_avgpool3d_large_batch() {
        let dev = get_test_device().await;
        
        // Multiple batches and channels (video-style)
        let batch_size = 2;
        let channels = 3;
        let depth = 8;
        let height = 8;
        let width = 8;
        
        let input: Vec<f32> = (0..batch_size * channels * depth * height * width)
            .map(|i| (i % 10) as f32)
            .collect();
        let output = avgpool3d(&dev.device, &dev.queue, &input, batch_size, channels, depth, height, width, 2, 2).await.unwrap();
        
        assert_eq!(output.len(), batch_size * channels * 4 * 4 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_avgpool3d_precision() {
        let dev = get_test_device().await;
        
        // Test with known values - 2×2×2 cube
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,  // 8 values in 2×2×2
        ];
        let output = avgpool3d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 2, 2, 2).await.unwrap();
        
        // Average of 1,2,3,4,5,6,7,8 = 36/8 = 4.5
        assert_eq!(output.len(), 1);
        assert!((output[0] - 4.5).abs() < 1e-6);
    }
}
