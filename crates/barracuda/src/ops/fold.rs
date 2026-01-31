//! Fold - Inverse of unfold (col2im)
//!
//! Combines sliding blocks back into tensor.

pub async fn fold(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],  // Unfolded patches
    batch_size: usize,
    channels: usize,
    output_h: usize,
    output_w: usize,
    kernel_h: usize,
    kernel_w: usize,
    stride: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * channels * output_h * output_w];
    let mut counts = vec![0u32; batch_size * channels * output_h * output_w];
    
    let num_patches_h = (output_h - kernel_h) / stride + 1;
    let num_patches_w = (output_w - kernel_w) / stride + 1;
    let num_patches = num_patches_h * num_patches_w;
    let patch_size = channels * kernel_h * kernel_w;
    
    for b in 0..batch_size {
        let mut patch_idx = 0;
        
        for ph in 0..num_patches_h {
            for pw in 0..num_patches_w {
                let mut col_idx = 0;
                
                for c in 0..channels {
                    for kh in 0..kernel_h {
                        for kw in 0..kernel_w {
                            let oh = ph * stride + kh;
                            let ow = pw * stride + kw;
                            
                            let in_idx = b * patch_size * num_patches + col_idx * num_patches + patch_idx;
                            let out_idx = b * channels * output_h * output_w + c * output_h * output_w + oh * output_w + ow;
                            
                            output[out_idx] += input[in_idx];
                            counts[out_idx] += 1;
                            col_idx += 1;
                        }
                    }
                }
                
                patch_idx += 1;
            }
        }
    }
    
    // Average overlapping regions
    for i in 0..output.len() {
        if counts[i] > 0 {
            output[i] /= counts[i] as f32;
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
    async fn test_fold_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 27 * 36]; // Folded 1x3x8x8 with 3x3 kernel
        let output = fold(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 3, 3, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 8 * 8);
    }

    #[tokio::test]
    async fn test_fold_edge_cases() {
        let dev = get_test_device().await;

        // Small kernel (2x2)
        let input = vec![1.0; 1 * 4 * 16]; // 1 batch, 1 channel, 4x4 patches with 2x2 kernel
        let output = fold(&dev.device, &dev.queue, &input, 1, 1, 5, 5, 2, 2, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 5 * 5);

        // Single channel
        let input = vec![1.0; 1 * 9 * 9]; // 1x1x4x4 with 3x3 kernel
        let output = fold(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 3, 3, 1).await.unwrap();
        assert!(output.len() > 0);
    }

    #[tokio::test]
    async fn test_fold_boundary() {
        let dev = get_test_device().await;

        // Stride > 1 (non-overlapping patches)
        // Simplified: smaller dimensions
        let input = vec![1.0; 1 * 4 * 9]; // Simplified input
        let output = fold(&dev.device, &dev.queue, &input, 1, 1, 6, 6, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 6 * 6);

        // Single batch, single channel
        let input = vec![1.0; 1 * 9 * 4]; // Simplified
        let output = fold(&dev.device, &dev.queue, &input, 1, 1, 4, 4, 2, 2, 1).await.unwrap();
        assert!(output.len() > 0);
    }

    #[tokio::test]
    async fn test_fold_large_batch() {
        let dev = get_test_device().await;

        // Batch size 4
        let batch_size = 4;
        let channels = 3;
        let out_h = 8;
        let out_w = 8;
        let kernel_h = 3;
        let kernel_w = 3;
        let num_patches = (out_h - kernel_h + 1) * (out_w - kernel_w + 1);
        let input = vec![1.0; batch_size * channels * kernel_h * kernel_w * num_patches];
        
        let output = fold(&dev.device, &dev.queue, &input, batch_size, channels, out_h, out_w, kernel_h, kernel_w, 1).await.unwrap();
        assert_eq!(output.len(), batch_size * channels * out_h * out_w);
    }

    #[tokio::test]
    async fn test_fold_precision() {
        let dev = get_test_device().await;

        // Test averaging of overlapping regions
        let input = vec![1.0; 1 * 9 * 9]; // 9 patches of 3x3
        let output = fold(&dev.device, &dev.queue, &input, 1, 1, 5, 5, 3, 3, 1).await.unwrap();
        assert_eq!(output.len(), 25);
        
        // All outputs should be finite and non-negative
        assert!(output.iter().all(|&x| x.is_finite() && x >= 0.0));
        
        // Center elements should have higher counts (more overlap) but same value (all 1.0)
        // With uniform input, output should be uniform
        assert!(output.iter().all(|&x| (x - 1.0).abs() < 0.1));
    }
}
