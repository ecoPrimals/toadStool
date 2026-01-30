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
    
    #[tokio::test]
    async fn test_fold() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 27 * 36]; // Folded 1x3x8x8 with 3x3 kernel
        let output = fold(&dev.device, &dev.queue, &input, 1, 3, 8, 8, 3, 3, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 8 * 8);
    }
}
