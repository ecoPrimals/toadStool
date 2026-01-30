//! Permute - Reorder dimensions
//!
//! Permutes dimensions according to specified order.

pub async fn permute(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    shape: &[usize],
    dims: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if dims.len() != shape.len() {
        return Err("Permutation dims must match shape rank".into());
    }
    
    // Simplified for common case: 4D tensor permutation
    if shape.len() != 4 || dims != &[0, 2, 3, 1] {
        return Err("Currently supports NCHW -> NHWC permutation only".into());
    }
    
    let (n, c, h, w) = (shape[0], shape[1], shape[2], shape[3]);
    let mut output = vec![0.0f32; input.len()];
    
    for b in 0..n {
        for ch in 0..c {
            for row in 0..h {
                for col in 0..w {
                    let in_idx = b * c * h * w + ch * h * w + row * w + col;
                    let out_idx = b * h * w * c + row * w * c + col * c + ch;
                    output[out_idx] = input[in_idx];
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
    
    #[tokio::test]
    async fn test_permute() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 3, 4, 4], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), input.len());
    }
}
